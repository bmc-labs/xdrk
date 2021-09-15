// Copyright 2021 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <alumni@bmc-labs.com>

use super::{bindings as aim, util, Channel, ChannelData, Lap, LapInfo};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use eyre::{bail, ensure, eyre, Result};
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use std::{cmp::Ordering,
          collections::HashMap,
          ffi::CStr,
          path::{Path, PathBuf},
          sync::{Arc, Mutex, Weak}};


lazy_static! {
  static ref LIBCALL_MTX: Mutex<HashMap<PathBuf, Weak<Run>>> =
    Mutex::new(HashMap::new());
}


/// Holds access information for the file and provides access to it.
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct Run {
  #[getset(get = "pub")]
  path:                   PathBuf,
  #[getset(get_copy = "pub")]
  idx:                    usize,
  #[getset(get_copy = "pub")]
  number_of_laps:         usize,
  #[getset(get = "pub")]
  info_of_laps:           Vec<LapInfo>,
  #[getset(get_copy = "pub")]
  number_of_channels:     usize,
  #[getset(get = "pub")]
  channel_names:          Vec<String>,
  #[getset(get = "pub")]
  channel_units:          Vec<String>,
  #[getset(get_copy = "pub")]
  channels_count:         usize,
  #[getset(get_copy = "pub")]
  gps_channels_count:     usize,
  #[getset(get_copy = "pub")]
  gps_raw_channels_count: usize,
}

// DESTRUCTOR - CLOSES FILE ------------------------------------------------ //
impl Drop for Run {
  /// Close the drk/xrk file on `Run` destruction
  fn drop(&mut self) {
    let mut loaded_paths = LIBCALL_MTX.lock().unwrap();
    unsafe { aim::close_file_i(self.idx as i32) };
    loaded_paths.remove(&self.path);
  }
}

impl Run {
  // META FUNCTIONS -------------------------------------------------------- //
  /// Library compilation date.
  pub fn library_date() -> Result<NaiveDate> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    Ok(NaiveDate::parse_from_str(unsafe {
                                   CStr::from_ptr(aim::get_library_date())
                                 }.to_str()?,
                                 "%b %d %Y")?)
  }

  /// Library compilation time.
  pub fn library_time() -> Result<NaiveTime> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    Ok(NaiveTime::parse_from_str(unsafe {
                                   CStr::from_ptr(aim::get_library_time())
                                 }.to_str()?,
                                 "%H:%M:%S")?)
  }

  /// Library compilation date and time.
  ///
  /// This is a convenience function wrapping the functions `library_date` and
  /// `library_time` to produce a datetime object.
  pub fn library_datetime() -> Result<NaiveDateTime> {
    Ok(Self::library_date()?.and_time(Self::library_time()?))
  }

  // FILE OPENING / CLOSING FUNCTIONS -------------------------------------- //
  /// Loads a drk/xrk file and creates an `Run` object.
  pub fn load(path: &Path) -> Result<Arc<Self>> {
    let extension =
      path.extension()
          .unwrap_or_default()
          .to_str()
          .ok_or(eyre!("file extension is not valid unicode ({})",
                       path.display()))?;

    ensure!(path.exists() && path.is_file(),
            "path does not exist or is not a valid file ({})",
            path.display());
    ensure!(["drk", "xrk"].contains(&extension),
            "only files with extensions .xrk and .drk accepted ({})",
            path.display());

    let mut loaded_paths = LIBCALL_MTX.lock().unwrap();
    let file = if !loaded_paths.contains_key(path) {
      // we need to open the file up
      let raw_idx =
        unsafe { aim::open_file(util::path_to_cstring(path)?.as_ptr()) };

      // match the index to check if opening file was a success
      let idx = match raw_idx.cmp(&0) {
        Ordering::Greater => raw_idx as usize,
        Ordering::Equal => {
          bail!("file is open but can't be parsed ({})", path.display())
        }
        Ordering::Less => bail!("an error occurred ({})", path.display()),
      };

      // get number of laps to cache it in `Run` object
      let count = unsafe { aim::get_laps_count(idx as i32) };
      let number_of_laps = match count.cmp(&0) {
        Ordering::Greater => count as usize,
        Ordering::Equal => bail!("file contains 0 laps ({})", path.display()),
        Ordering::Less => {
          bail!("error getting lap count ({})", path.display())
        }
      };

      // get info of all laps to cache it in `Run` object
      let mut info_of_laps = Vec::with_capacity(number_of_laps);
      for lap_idx in 0..number_of_laps {
        let (mut start, mut time) = (0.0f64, 0.0f64);
        let err_code = unsafe {
          aim::get_lap_info(idx as i32, lap_idx as i32, &mut start, &mut time)
        };
        ensure!(err_code == 1,
                "could not fetch lap info ({})",
                path.display());
        info_of_laps.push(LapInfo::new(lap_idx, start, time));
      }

      // get count of channels to cache it in `Run` object
      let count = unsafe { aim::get_channels_count(idx as i32) };
      let channels_count = match count.cmp(&0) {
        Ordering::Greater => count as usize,
        Ordering::Equal => {
          bail!("file contains 0 channels ({})", path.display())
        }
        Ordering::Less => {
          bail!("error getting channel count ({})", path.display())
        }
      };

      // get count of gps channels to cache it in `Run` object
      let count = unsafe { aim::get_GPS_channels_count(idx as i32) };
      let gps_channels_count = match count.cmp(&0) {
        Ordering::Greater | Ordering::Equal => count as usize,
        Ordering::Less => {
          bail!("error getting GPS channel count ({})", path.display())
        }
      };

      // get count of gps raw channels to cache it in `Run` object
      let count = unsafe { aim::get_GPS_raw_channels_count(idx as i32) };
      let gps_raw_channels_count = match count.cmp(&0) {
        Ordering::Greater | Ordering::Equal => count as usize,
        Ordering::Less => {
          bail!("error getting GPS raw channel count ({})", path.display())
        }
      };

      // the magic in the following section is this:
      //
      // WE ONLY TAKE THE REGULAR CHANNELS AND THE GPS CHANNELS AND OMIT THE
      // "RAW" GPS CHANNELS FROM THE DATA, SINCE WE ARE GENERALLY NOT
      // INTERESTED IN THOSE. to make that happen we simply don't count the raw
      // channels and don't get their names nor units. as such, when the `Run`
      // object is asked for `number_of_channels`, or `channel_names` or
      // `channel_units`, the raw channels are simply not included.
      //
      // get total number of channels to cache it in `Run` object
      let number_of_channels = channels_count + gps_channels_count;

      // get channel names to cache it in `Run` object
      let mut channel_names = Vec::with_capacity(number_of_channels);

      for channel_idx in 0..number_of_channels {
        let name = util::strptr_to_string(unsafe {
          if channel_idx < channels_count {
            aim::get_channel_name(idx as i32, channel_idx as i32)
          } else {
            let channel_idx = channel_idx % channels_count;
            aim::get_GPS_channel_name(idx as i32, channel_idx as i32)
          }
        })?;
        channel_names.push(name);
      }

      // get channel units to cache it in `Run` object
      let mut channel_units = Vec::with_capacity(number_of_channels);
      for channel_idx in 0..number_of_channels {
        let unit = util::strptr_to_string(unsafe {
          if channel_idx < channels_count {
            aim::get_channel_units(idx as i32, channel_idx as i32)
          } else {
            let channel_idx = channel_idx % channels_count;
            aim::get_GPS_channel_units(idx as i32, channel_idx as i32)
          }
        })?;
        channel_units.push(unit);
      }

      // create object in Arc, pass it as weak pointer into global mutex
      let file = Arc::new(Self { path: path.to_owned(),
                                 idx,
                                 number_of_laps,
                                 info_of_laps,
                                 number_of_channels,
                                 channel_names,
                                 channel_units,
                                 channels_count,
                                 gps_channels_count,
                                 gps_raw_channels_count });

      loaded_paths.insert(path.to_owned(), Arc::downgrade(&file));
      file
    } else {
      loaded_paths.get(path)
                  .expect("failed to get Arc object in load function")
                  .upgrade()
                  .unwrap()
    };

    // return Arc, incrementing the ref counter on the object
    Ok(file)
  }

  // RUN LEVEL FUNCTIONS --------------------------------------------------- //
  pub fn championship(&self) -> Result<String> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_championship_name(self.idx as i32)
    })
  }

  pub fn track(&self) -> Result<String> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe { aim::get_track_name(self.idx as i32) })
  }

  pub fn venue_type(&self) -> Result<String> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_venue_type_name(self.idx as i32)
    })
  }

  pub fn vehicle(&self) -> Result<String> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe { aim::get_vehicle_name(self.idx as i32) })
  }

  pub fn racer(&self) -> Result<String> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe { aim::get_racer_name(self.idx as i32) })
  }

  /// On success, the `Result` contains a datetime object which defines when
  /// this `Run` was recorded.
  pub fn datetime(&self) -> Result<NaiveDateTime> {
    let _guard = LIBCALL_MTX.lock().unwrap();
    let tm: *const aim::tm =
      unsafe { aim::get_date_and_time(self.idx as i32) };
    ensure!(!tm.is_null(), "could not fetch datetime object");

    let tm = unsafe { *tm };
    Ok(NaiveDate::from_ymd(tm.tm_year + 1900,
                           (tm.tm_mon + 1) as u32,
                           tm.tm_mday as u32).and_hms(tm.tm_hour as u32,
                                                      tm.tm_min as u32,
                                                      tm.tm_sec as u32))
  }

  /// For lap with index `lap_idx`, request `LapInfo`. Returns an error if
  /// `lap_idx` is out of range (i.e. the `Run` does not contain a lap
  /// with that index) or the library calls fails for any reason.
  ///
  /// `LapInfo` objects contain the lap number, the start of the lap within the
  /// run recorded in this file (via the `start()` getter) and the lap duration
  /// (via the `duration()` getter).
  pub fn lap_info(&self, lap_idx: usize) -> Result<LapInfo> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    Ok(self.info_of_laps[lap_idx])
  }

  /// For lap with index `idx`, request all channels. Returns a Lap object or
  /// an error if `idx` is out of range (i.e. the `Run` does not contain a
  /// lap with that index) or the library call fails for any reason.
  ///
  /// `Lap` objects contain a `LapInfo` object and a `Vec<Channel>` containing
  /// all data recorded in the lap.
  pub fn lap(&self, lap_idx: usize) -> Result<Lap> {
    let len = self.number_of_channels;
    let mut channels = Vec::with_capacity(len);
    for channel_idx in 0..len {
      channels.push(self.channel(channel_idx, Some(lap_idx))?);
    }
    Ok(Lap::new(self.lap_info(lap_idx)?, channels))
  }

  /// Request all channels for all laps contained in this `Run`. Fails if
  /// the library call fails for any reason, either on finding all laps or on
  /// requesting `Lap`s.
  pub fn all_laps(&self) -> Result<Vec<Lap>> {
    let len = self.number_of_laps();
    let mut laps = Vec::with_capacity(len);
    for lap_idx in 0..len {
      laps.push(self.lap(lap_idx)?);
    }
    Ok(laps)
  }

  /// For channel with index `idx`, request the channel name.
  pub fn channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.number_of_channels,
            "channel_idx out of range");
    Ok(self.channel_names[channel_idx].clone())
  }

  /// Request index of channel with name `channel_name`.
  pub fn channel_idx(&self, channel_name: &str) -> Result<usize> {
    let channel_idx =
      self.channel_names
          .iter()
          .position(|name| name == channel_name)
          .ok_or(eyre!("no channel '{}' found", channel_name))?;

    assert!(channel_idx < self.number_of_channels,
            "channel index out of range");
    Ok(channel_idx)
  }

  /// For channel with index `channel_idx`, request the channel unit.
  pub fn channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.number_of_channels,
            "channel_idx out of range");
    Ok(self.channel_units[channel_idx].clone())
  }

  /// Request a `Channel` object by name and lap index. Fails if no channel
  /// with that name exists, if no lap with that index exists or the library
  /// call fails for any reason. Pass `None` for lap to get the raw channel
  /// with data from all laps.
  pub fn channel(&self,
                 channel_idx: usize,
                 lap_idx: Option<usize>)
                 -> Result<Channel> {
    Ok(Channel::new(self.channel_name(channel_idx)?,
                    self.channel_unit(channel_idx)?,
                    self.channel_data(channel_idx, lap_idx)?))
  }

  /// For channel with id `channel_id`, collect the measurement samples in a
  /// `ChannelData` object. GPS data included.
  pub fn channel_data(&self,
                      channel_idx: usize,
                      lap_idx: Option<usize>)
                      -> Result<ChannelData> {
    if let Some(lap_idx) = lap_idx {
      if channel_idx < self.channels_count {
        self.lap_channel_samples(lap_idx, channel_idx)
      } else {
        let channel_idx = channel_idx % self.channels_count;
        self.lap_gps_channel_samples(lap_idx, channel_idx)
      }
    } else {
      if channel_idx < self.channels_count {
        self.channel_samples(channel_idx)
      } else {
        let channel_idx = channel_idx % self.channels_count;
        self.gps_channel_samples(channel_idx)
      }
    }
  }

  // ----------------------------------------------------------------------- //
  // RAW LIBRARY FUNCTIONS ------------------------------------------------- //
  // ----------------------------------------------------------------------- //
  /// For channel with index `channel_idx`, request the number of samples
  /// contained in this `Run`.
  pub fn channel_samples_count(&self, channel_idx: usize) -> Result<usize> {
    ensure!(channel_idx < self.channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_channel_samples_count(self.idx as i32, channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("channel contains 0 samples"),
      Ordering::Less => bail!("error getting channel samples count"),
    }
  }

  /// For channel with index `channel_idx`, request the samples contained in
  /// this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn channel_samples(&self, channel_idx: usize) -> Result<ChannelData> {
    ensure!(channel_idx < self.channels_count,
            "channel_idx out of range");

    let count = self.channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_channel_samples(self.idx as i32,
                               channel_idx as i32,
                               timestamps.as_mut_ptr(),
                               samples.as_mut_ptr(),
                               count as i32)
    };
    ensure!(read == count as i32, "error reading channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and channel with index `channel_idx`,
  /// request the number of samples contained in this `Run`.
  pub fn lap_channel_samples_count(&self,
                                   lap_idx: usize,
                                   channel_idx: usize)
                                   -> Result<usize> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_lap_channel_samples_count(self.idx as i32,
                                         lap_idx as i32,
                                         channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("channel contains 0 samples in this lap"),
      Ordering::Less => bail!("error getting lap channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and channel with index `channel_idx`,
  /// request the samples contained in this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_channel_samples(&self,
                             lap_idx: usize,
                             channel_idx: usize)
                             -> Result<ChannelData> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count,
            "channel_idx out of range");

    let count = self.lap_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_lap_channel_samples(self.idx as i32,
                                   lap_idx as i32,
                                   channel_idx as i32,
                                   timestamps.as_mut_ptr(),
                                   samples.as_mut_ptr(),
                                   count as i32)
    };
    ensure!(read == count as i32, "error reading lap channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  // GPS INFORMATION FUNCTIONS --------------------------------------------- //
  //
  // GPS channels are the same channels added to AiM drk files in RS2Analysis,
  // those that consider vehicle dynamics assuming that the vehicle is
  // constantly aligned to the trajectory.
  //
  /// For GPS channel with index `channel_idx`, request the channel name.
  pub fn gps_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_GPS_channel_name(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS channel with index `channel_idx`, request the GPS channel unit.
  pub fn gps_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_GPS_channel_units(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS channel with index `channel_idx`, request the number of samples
  /// contained in this `Run`.
  pub fn gps_channel_samples_count(&self,
                                   channel_idx: usize)
                                   -> Result<usize> {
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_GPS_channel_samples_count(self.idx as i32, channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("GPS channel contains 0 samples"),
      Ordering::Less => bail!("error getting GPS channel samples count"),
    }
  }

  /// For GPS channel with index `channel_idx`, request the samples contained
  /// in this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn gps_channel_samples(&self,
                             channel_idx: usize)
                             -> Result<ChannelData> {
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let count = self.gps_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_GPS_channel_samples(self.idx as i32,
                                   channel_idx as i32,
                                   timestamps.as_mut_ptr(),
                                   samples.as_mut_ptr(),
                                   count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and GPS channel with index `channel_idx`,
  /// request the number of samples contained in this `Run`.
  pub fn lap_gps_channel_samples_count(&self,
                                       lap_idx: usize,
                                       channel_idx: usize)
                                       -> Result<usize> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_lap_GPS_channel_samples_count(self.idx as i32,
                                             lap_idx as i32,
                                             channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("GPS channel contains 0 samples in this lap"),
      Ordering::Less => bail!("error getting lap GPS channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and GPS channel with index `channel_idx`,
  /// request the samples contained in this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_gps_channel_samples(&self,
                                 lap_idx: usize,
                                 channel_idx: usize)
                                 -> Result<ChannelData> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count,
            "channel_idx out of range");

    let count = self.lap_gps_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_lap_GPS_channel_samples(self.idx as i32,
                                       lap_idx as i32,
                                       channel_idx as i32,
                                       timestamps.as_mut_ptr(),
                                       samples.as_mut_ptr(),
                                       count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  // ----------------------------------------------------------------------- //
  // RAW GPS FUNCTIONS ----------------------------------------------------- //
  // ----------------------------------------------------------------------- //
  /// For GPS raw channel with index `channel_idx`, request the channel name.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_name(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the GPS channel
  /// unit.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    util::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_units(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the number of
  /// samples contained in this `Run`.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_samples_count(&self,
                                       channel_idx: usize)
                                       -> Result<usize> {
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_GPS_raw_channel_samples_count(self.idx as i32,
                                             channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("GPS channel contains 0 samples"),
      Ordering::Less => bail!("error getting GPS channel samples count"),
    }
  }

  /// For GPS raw channel with index `channel_idx`, request the samples
  /// contained in this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_samples(&self,
                                 channel_idx: usize)
                                 -> Result<ChannelData> {
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let count = self.gps_raw_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_GPS_raw_channel_samples(self.idx as i32,
                                       channel_idx as i32,
                                       timestamps.as_mut_ptr(),
                                       samples.as_mut_ptr(),
                                       count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and GPS raw channel with index
  /// `channel_idx`, request the number of samples contained in this
  /// `Run`.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn lap_gps_raw_channel_samples_count(&self,
                                           lap_idx: usize,
                                           channel_idx: usize)
                                           -> Result<usize> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let _guard = LIBCALL_MTX.lock().unwrap();
    let count = unsafe {
      aim::get_lap_GPS_raw_channel_samples_count(self.idx as i32,
                                                 lap_idx as i32,
                                                 channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("GPS channel contains 0 samples in this lap"),
      Ordering::Less => bail!("error getting lap GPS channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and GPS raw channel with index
  /// `channel_idx`, request the samples contained in this `Run`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn lap_gps_raw_channel_samples(&self,
                                     lap_idx: usize,
                                     channel_idx: usize)
                                     -> Result<ChannelData> {
    ensure!(lap_idx < self.number_of_laps, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count,
            "channel_idx out of range");

    let count = self.lap_gps_raw_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let _guard = LIBCALL_MTX.lock().unwrap();
    let read = unsafe {
      aim::get_lap_GPS_raw_channel_samples(self.idx as i32,
                                           lap_idx as i32,
                                           channel_idx as i32,
                                           timestamps.as_mut_ptr(),
                                           samples.as_mut_ptr(),
                                           count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }
  // ----------------------------------------------------------------------- //
}
// LIBRARY CODE END -------------------------------------------------------- //


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use std::fs;


  const XRK_PATH: &str =
    "./testdata/032/TCR_EU-21_E02-LCA_Q1_AU-RS3-R5-S-S_032_A_1375.xrk";

  #[cfg(target_family = "windows")]
  const DRK_PATH: &str =
    "./testdata/032/TCR_EU-21_E02-LCA_Q1_AU-RS3-R5-S-S_032_A_1375.drk";

  #[test]
  #[ignore]
  fn drop_test() {
    // opening XRK and DRK files produces temporary files which are cleaned
    // up when the file is closed, which we do via `Drop` so it happens when
    // the object goes out of scope. to test this is working, we wrap the
    // actual test in a block...
    {
      let xdrk_file = Run::load(Path::new(XRK_PATH)).unwrap();
      assert!(xdrk_file.idx() > 0);
    }
    // ... and then scan for temporary files afterwards
    let allowed_extensions = vec!["drk", "rrk", "xrk", "xrz"];
    for file in fs::read_dir(Path::new("./testdata")).unwrap() {
      let file = file.unwrap();
      assert_eq!(true,
                 allowed_extensions.contains(&file.path()
                                                  .extension()
                                                  .unwrap()
                                                  .to_str()
                                                  .unwrap()));
    }
  }

  #[test]
  fn xrkfile_test() {
    let xdrk_file = Run::load(Path::new(XRK_PATH)).unwrap();
    // println!("{:#?}", xdrk_file.championship());
    // println!("{:#?}", xdrk_file.vehicle());
    // println!("{:#?}", xdrk_file.racer());
    // println!("{:#?}", xdrk_file.venue_type());
    // println!("{:#?}", xdrk_file.track());
    // println!("{:#?}", xdrk_file.datetime());
    // println!("{:#?}", xdrk_file.channel_names());
    // println!("{:#?}", xdrk_file.channel_units());

    assert_eq!("TCR_EU-21_E02-LCA", &xdrk_file.championship().unwrap());
    assert_eq!("AU-RS3-R5-S-S", &xdrk_file.vehicle().unwrap());
    assert_eq!("032", &xdrk_file.racer().unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("Q1", &xdrk_file.venue_type().unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("Qualifying 1", &xdrk_file.venue_type().unwrap());

    assert_eq!("TCR_LCA_2.0", &xdrk_file.track().unwrap());
    assert_eq!(NaiveDate::from_ymd(2021, 05, 29).and_hms(09, 59, 44),
               xdrk_file.datetime().unwrap());

    assert_eq!(5, xdrk_file.number_of_laps());
    assert_eq!(LapInfo::new(2, 336.179, 134.718),
               xdrk_file.lap_info(2).unwrap());

    assert_eq!(50, xdrk_file.number_of_channels());

    macro_rules! stringvec {
      ($($x:literal),* $(,)?) => (vec![$($x.to_string()),*]);
    }
    #[cfg(target_family = "unix")]
    let channel_names = stringvec!["Logger Temperature",
                                   "External Voltage",
                                   "pManifoldScrut",
                                   "tManifoldScrut",
                                   "aLon",
                                   "aLat",
                                   "aVer",
                                   "wRoll",
                                   "wPitch",
                                   "wYaw",
                                   "bAdvance",
                                   "bSteering",
                                   "bVvtIn",
                                   "bVvtOut",
                                   "dInjection",
                                   "fEngRpm",
                                   "pBrakeF",
                                   "pBrakeR",
                                   "pManifold",
                                   "posGear",
                                   "pRail",
                                   "rLambda",
                                   "rPedal",
                                   "rThrottle",
                                   "swGearDown",
                                   "swGearUp",
                                   "swLaunchState",
                                   "swRotFcy",
                                   "swRotPit",
                                   "tAmbient",
                                   "tManifold",
                                   "tWater",
                                   "uBarrel",
                                   "vWheelFL",
                                   "vWheelFR",
                                   "vWheelRL",
                                   "vWheelRR",
                                   "momEngTorq",
                                   "momEngTorqTarget",
                                   "GPS Speed",
                                   "GPS Nsat",
                                   "GPS LatAcc",
                                   "GPS LonAcc",
                                   "GPS Slope",
                                   "GPS Heading",
                                   "GPS Gyro",
                                   "GPS Altitude",
                                   "GPS PosAccuracy",
                                   "GPS SpdAccuracy",
                                   "GPS Radius",];
    #[cfg(target_family = "windows")]
    let channel_names = stringvec!["Logger Temperature",
                                   "External Voltage",
                                   "pManifoldScrut",
                                   "tManifoldScrut",
                                   "aLon",
                                   "aLat",
                                   "aVer",
                                   "wRoll",
                                   "wPitch",
                                   "wYaw",
                                   "bAdvance",
                                   "bSteering",
                                   "bVvtIn",
                                   "bVvtOut",
                                   "dInjection",
                                   "fEngRpm",
                                   "pBrakeF",
                                   "pBrakeR",
                                   "pManifold",
                                   "posGear",
                                   "pRail",
                                   "rLambda",
                                   "rPedal",
                                   "rThrottle",
                                   "swGearDown",
                                   "swGearUp",
                                   "swLaunchState",
                                   "swRotFcy",
                                   "swRotPit",
                                   "tAmbient",
                                   "tManifold",
                                   "tWater",
                                   "uBarrel",
                                   "vWheelFL",
                                   "vWheelFR",
                                   "vWheelRL",
                                   "vWheelRR",
                                   "momEngTorq",
                                   "momEngTorqTarget",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",];

    assert_eq!(&channel_names, xdrk_file.channel_names());

    assert_eq!("Logger Temperature", &xdrk_file.channel_name(0).unwrap());
    assert_eq!("pManifoldScrut", &xdrk_file.channel_name(2).unwrap());
    assert_eq!("fEngRpm", &xdrk_file.channel_name(15).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("GPS Speed", &xdrk_file.channel_name(39).unwrap());
    #[cfg(target_family = "unix")]
    assert_eq!("GPS Nsat", &xdrk_file.channel_name(40).unwrap());

    #[cfg(target_family = "windows")]
    assert_eq!("", &xdrk_file.channel_name(39).unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("", &xdrk_file.channel_name(40).unwrap());

    assert_eq!("C", &xdrk_file.channel_unit(0).unwrap());
    assert_eq!("bar", &xdrk_file.channel_unit(2).unwrap());
    assert_eq!("rpm", &xdrk_file.channel_unit(15).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("m/s", &xdrk_file.channel_unit(39).unwrap());
    #[cfg(target_family = "unix")]
    assert_eq!("#", &xdrk_file.channel_unit(40).unwrap());

    #[cfg(target_family = "windows")]
    assert_eq!("", &xdrk_file.channel_unit(39).unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("", &xdrk_file.channel_unit(40).unwrap());

    assert_eq!(672, xdrk_file.channel_samples_count(0).unwrap());
    assert_eq!(70588, xdrk_file.channel_samples_count(2).unwrap());
    assert_eq!(70547, xdrk_file.channel_samples_count(15).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!(70620, xdrk_file.gps_channel_samples_count(0).unwrap());
    #[cfg(target_family = "unix")]
    assert_eq!(70620, xdrk_file.gps_channel_samples_count(1).unwrap());

    #[cfg(target_family = "windows")]
    assert_eq!(70700, xdrk_file.gps_channel_samples_count(0).unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!(70700, xdrk_file.gps_channel_samples_count(1).unwrap());

    assert_eq!(false, xdrk_file.channel_samples(0).unwrap().is_empty());
    assert_eq!(128, xdrk_file.lap_channel_samples_count(2, 0).unwrap());
    assert_eq!(false,
               xdrk_file.lap_channel_samples(2, 0).unwrap().is_empty());

    #[cfg(target_family = "unix")]
    {
      let correct_channel = "GPS Radius";
      assert_eq!(49, xdrk_file.channel_idx(&correct_channel).unwrap());

      let wrong_channel = "wrong channel";
      assert_eq!(true, xdrk_file.channel_idx(&wrong_channel).is_err());

      assert_eq!("GPS Speed", &xdrk_file.gps_channel_name(0).unwrap());
      assert_eq!("GPS LatAcc", &xdrk_file.gps_channel_name(2).unwrap());
      assert_eq!("GPS Gyro", &xdrk_file.gps_channel_name(6).unwrap());
      assert_eq!("GPS PosAccuracy", &xdrk_file.gps_channel_name(8).unwrap());
      assert_eq!("GPS Radius", &xdrk_file.gps_channel_name(10).unwrap());
    }

    assert_eq!("ECEF position_X",
               &xdrk_file.gps_raw_channel_name(0).unwrap());
    assert_eq!("ECEF position_Y",
               &xdrk_file.gps_raw_channel_name(1).unwrap());
    assert_eq!("ECEF velocity_Y",
               &xdrk_file.gps_raw_channel_name(4).unwrap());
    assert_eq!("N Satellites", &xdrk_file.gps_raw_channel_name(6).unwrap());
    assert_eq!("Week N", &xdrk_file.gps_raw_channel_name(8).unwrap());

    assert_eq!("m", &xdrk_file.gps_raw_channel_unit(0).unwrap());
    assert_eq!("m", &xdrk_file.gps_raw_channel_unit(1).unwrap());
    assert_eq!("m/s", &xdrk_file.gps_raw_channel_unit(3).unwrap());
    assert_eq!("ms", &xdrk_file.gps_raw_channel_unit(7).unwrap());
    assert_eq!("#", &xdrk_file.gps_raw_channel_unit(8).unwrap());

    assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(0).unwrap());
    assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(3).unwrap());
    assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(4).unwrap());
    assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(6).unwrap());
    assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(8).unwrap());

    assert_eq!(false,
               xdrk_file.gps_raw_channel_samples(0).unwrap().is_empty());
    assert_eq!(false, xdrk_file.gps_channel_samples(0).unwrap().is_empty());

    #[cfg(target_family = "unix")]
    {
      assert_eq!("m/s", &xdrk_file.gps_channel_unit(0).unwrap());
      assert_eq!("g", &xdrk_file.gps_channel_unit(2).unwrap());
      assert_eq!("deg", &xdrk_file.gps_channel_unit(5).unwrap());
      assert_eq!("#", &xdrk_file.gps_channel_unit(8).unwrap());
      assert_eq!("m", &xdrk_file.gps_channel_unit(10).unwrap());
    }

    assert_eq!(2010,
               xdrk_file.lap_gps_raw_channel_samples_count(0, 0).unwrap());
    assert_eq!(1346,
               xdrk_file.lap_gps_raw_channel_samples_count(2, 0).unwrap());
    assert_eq!(1599,
               xdrk_file.lap_gps_raw_channel_samples_count(3, 0).unwrap());
    assert_eq!(1348,
               xdrk_file.lap_gps_raw_channel_samples_count(1, 1).unwrap());
    assert_eq!(1346,
               xdrk_file.lap_gps_raw_channel_samples_count(2, 2).unwrap());

    assert_eq!(false,
               xdrk_file.lap_gps_raw_channel_samples(2, 0)
                        .unwrap()
                        .is_empty());
    assert_eq!(false,
               xdrk_file.lap_gps_channel_samples(0, 0).unwrap().is_empty());
  }

  #[cfg(target_family = "windows")]
  #[test]
  fn drkfile_test() {
    let xdrk_file = Run::load(Path::new(DRK_PATH)).unwrap();

    assert_eq!("TCR_EU-21_E02-LCA", &xdrk_file.championship().unwrap());
    assert_eq!("AU-RS3-R5-S-S", &xdrk_file.vehicle().unwrap());
    assert_eq!("032", &xdrk_file.racer().unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("Q1", &xdrk_file.venue_type().unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("Qualifying 1", &xdrk_file.venue_type().unwrap());

    assert_eq!("TCR_LCA_2.0", &xdrk_file.track().unwrap());
    assert_eq!(NaiveDate::from_ymd(2021, 05, 29).and_hms(09, 59, 44),
               xdrk_file.datetime().unwrap());

    assert_eq!(11, xdrk_file.number_of_laps());
    assert_eq!(LapInfo::new(2, 336.179, 134.718),
               xdrk_file.lap_info(2).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!(49, xdrk_file.number_of_channels());
    #[cfg(target_family = "windows")]
    assert_eq!(49, xdrk_file.number_of_channels());

    macro_rules! stringvec {
      ($($x:literal),* $(,)?) => (vec![$($x.to_string()),*]);
    }
    #[cfg(target_family = "unix")]
    let channel_names = stringvec!["Logger Temperature",
                                   "External Voltage",
                                   "pManifoldScrut",
                                   "tManifoldScrut",
                                   "aLon",
                                   "aLat",
                                   "aVer",
                                   "wRoll",
                                   "wPitch",
                                   "wYaw",
                                   "bAdvance",
                                   "bSteering",
                                   "bVvtIn",
                                   "bVvtOut",
                                   "dInjection",
                                   "fEngRpm",
                                   "pBrakeF",
                                   "pBrakeR",
                                   "pManifold",
                                   "posGear",
                                   "pRail",
                                   "rLambda",
                                   "rPedal",
                                   "rThrottle",
                                   "swGearDown",
                                   "swGearUp",
                                   "swLaunchState",
                                   "swRotFcy",
                                   "swRotPit",
                                   "tAmbient",
                                   "tManifold",
                                   "tWater",
                                   "uBarrel",
                                   "vWheelFL",
                                   "vWheelFR",
                                   "vWheelRL",
                                   "vWheelRR",
                                   "momEngTorq",
                                   "momEngTorqTarget",
                                   "GPS_Speed",
                                   "GPS_Nsat",
                                   "GPS_LatAcc",
                                   "GPS_LonAcc",
                                   "GPS_Slope",
                                   "GPS_Heading",
                                   "GPS_Gyro",
                                   "GPS_Altitude",
                                   "GPS_PosAccuracy",
                                   "GPS_SpdAccuracy",];
    #[cfg(target_family = "windows")]
    let channel_names = stringvec!["Logger Temperature",
                                   "External Voltage",
                                   "pManifoldScrut",
                                   "tManifoldScrut",
                                   "aLon",
                                   "aLat",
                                   "aVer",
                                   "wRoll",
                                   "wPitch",
                                   "wYaw",
                                   "bAdvance",
                                   "bSteering",
                                   "bVvtIn",
                                   "bVvtOut",
                                   "dInjection",
                                   "fEngRpm",
                                   "pBrakeF",
                                   "pBrakeR",
                                   "pManifold",
                                   "posGear",
                                   "pRail",
                                   "rLambda",
                                   "rPedal",
                                   "rThrottle",
                                   "swGearDown",
                                   "swGearUp",
                                   "swLaunchState",
                                   "swRotFcy",
                                   "swRotPit",
                                   "tAmbient",
                                   "tManifold",
                                   "tWater",
                                   "uBarrel",
                                   "vWheelFL",
                                   "vWheelFR",
                                   "vWheelRL",
                                   "vWheelRR",
                                   "momEngTorq",
                                   "momEngTorqTarget",
                                   "GPS_Speed",
                                   "GPS_Nsat",
                                   "GPS_LatAcc",
                                   "GPS_LonAcc",
                                   "GPS_Slope",
                                   "GPS_Heading",
                                   "GPS_Gyro",
                                   "GPS_Altitude",
                                   "GPS_PosAccuracy",
                                   "GPS_SpdAccuracy",];

    assert_eq!(&channel_names, xdrk_file.channel_names());

    assert_eq!("Logger Temperature", &xdrk_file.channel_name(0).unwrap());
    assert_eq!("pManifoldScrut", &xdrk_file.channel_name(2).unwrap());
    assert_eq!("fEngRpm", &xdrk_file.channel_name(15).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("GPS_Speed", &xdrk_file.channel_name(39).unwrap());
    #[cfg(target_family = "unix")]
    assert_eq!("GPS_Nsat", &xdrk_file.channel_name(40).unwrap());

    #[cfg(target_family = "windows")]
    assert_eq!("GPS_Speed", &xdrk_file.channel_name(39).unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("GPS_Nsat", &xdrk_file.channel_name(40).unwrap());

    // assert_eq!("C", &xdrk_file.channel_unit(0).unwrap());
    assert_eq!("mbar", &xdrk_file.channel_unit(2).unwrap());
    assert_eq!("rpm", &xdrk_file.channel_unit(15).unwrap());

    #[cfg(target_family = "unix")]
    assert_eq!("m/s", &xdrk_file.channel_unit(39).unwrap());
    #[cfg(target_family = "unix")]
    assert_eq!("#", &xdrk_file.channel_unit(40).unwrap());

    #[cfg(target_family = "windows")]
    assert_eq!("km/h", &xdrk_file.channel_unit(39).unwrap());
    #[cfg(target_family = "windows")]
    assert_eq!("#", &xdrk_file.channel_unit(40).unwrap());

    assert_eq!(1554, xdrk_file.channel_samples_count(0).unwrap());
    assert_eq!(155400, xdrk_file.channel_samples_count(2).unwrap());
    assert_eq!(155400, xdrk_file.channel_samples_count(15).unwrap());

    assert_eq!(false, xdrk_file.channel_samples(0).unwrap().is_empty());
    assert_eq!(134, xdrk_file.lap_channel_samples_count(2, 0).unwrap());
    assert_eq!(false,
               xdrk_file.lap_channel_samples(2, 0).unwrap().is_empty());

    #[cfg(target_family = "unix")]
    {
      let correct_channel = "GPS Radius";
      assert_eq!(49, xdrk_file.channel_idx(&correct_channel).unwrap());

      let wrong_channel = "wrong channel";
      assert_eq!(true, xdrk_file.channel_idx(&wrong_channel).is_err());

      assert_eq!("GPS Speed", &xdrk_file.gps_channel_name(0).unwrap());
      assert_eq!("GPS LatAcc", &xdrk_file.gps_channel_name(2).unwrap());
      assert_eq!("GPS Gyro", &xdrk_file.gps_channel_name(6).unwrap());
      assert_eq!("GPS PosAccuracy", &xdrk_file.gps_channel_name(8).unwrap());
      assert_eq!("GPS Radius", &xdrk_file.gps_channel_name(10).unwrap());

      assert_eq!("ECEF position_X",
                 &xdrk_file.gps_raw_channel_name(0).unwrap());
      assert_eq!("ECEF position_Y",
                 &xdrk_file.gps_raw_channel_name(1).unwrap());
      assert_eq!("ECEF velocity_Y",
                 &xdrk_file.gps_raw_channel_name(4).unwrap());
      assert_eq!("N Satellites", &xdrk_file.gps_raw_channel_name(6).unwrap());
      assert_eq!("Week N", &xdrk_file.gps_raw_channel_name(8).unwrap());

      assert_eq!("m", &xdrk_file.gps_raw_channel_unit(0).unwrap());
      assert_eq!("m", &xdrk_file.gps_raw_channel_unit(1).unwrap());
      assert_eq!("m/s", &xdrk_file.gps_raw_channel_unit(3).unwrap());
      assert_eq!("ms", &xdrk_file.gps_raw_channel_unit(7).unwrap());
      assert_eq!("#", &xdrk_file.gps_raw_channel_unit(8).unwrap());

      assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(0).unwrap());
      assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(3).unwrap());
      assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(4).unwrap());
      assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(6).unwrap());
      assert_eq!(7061, xdrk_file.gps_raw_channel_samples_count(8).unwrap());

      assert_eq!(false,
                 xdrk_file.gps_raw_channel_samples(0).unwrap().is_empty());
      assert_eq!(false, xdrk_file.gps_channel_samples(0).unwrap().is_empty());

      assert_eq!("m/s", &xdrk_file.gps_channel_unit(0).unwrap());
      assert_eq!("g", &xdrk_file.gps_channel_unit(2).unwrap());
      assert_eq!("deg", &xdrk_file.gps_channel_unit(5).unwrap());
      assert_eq!("#", &xdrk_file.gps_channel_unit(8).unwrap());
      assert_eq!("m", &xdrk_file.gps_channel_unit(10).unwrap());

      assert_eq!(2010,
                 xdrk_file.lap_gps_raw_channel_samples_count(0, 0).unwrap());
      assert_eq!(1346,
                 xdrk_file.lap_gps_raw_channel_samples_count(2, 0).unwrap());
      assert_eq!(1599,
                 xdrk_file.lap_gps_raw_channel_samples_count(3, 0).unwrap());
      assert_eq!(1348,
                 xdrk_file.lap_gps_raw_channel_samples_count(1, 1).unwrap());
      assert_eq!(1346,
                 xdrk_file.lap_gps_raw_channel_samples_count(2, 2).unwrap());

      assert_eq!(false,
                 xdrk_file.lap_gps_raw_channel_samples(2, 0)
                          .unwrap()
                          .is_empty());
      assert_eq!(false,
                 xdrk_file.lap_gps_channel_samples(0, 0).unwrap().is_empty());
    }
  }

  #[test]
  fn meta_fn() {
    let (date, time) = {
      #[cfg(target_family = "unix")]
      let date = NaiveDate::from_ymd(2020, 1, 24);
      #[cfg(target_family = "windows")]
      let date = NaiveDate::from_ymd(2021, 5, 4);

      #[cfg(target_family = "unix")]
      let time = NaiveTime::from_hms(16, 36, 19);
      #[cfg(target_family = "windows")]
      let time = NaiveTime::from_hms(12, 50, 34);

      (date, time)
    };

    assert_eq!(date.and_time(time), Run::library_datetime().unwrap());
    assert_eq!(date, Run::library_date().unwrap());
    assert_eq!(time, Run::library_time().unwrap());
  }
}
