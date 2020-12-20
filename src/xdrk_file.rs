// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <jonas@bmc-labs.com>

use super::{service as srv,
            xdrk_bindings as aim,
            Lap,
            LapInfo,
            RawChannel,
            RawChannelData};
use anyhow::{anyhow, bail, ensure, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use getset::{CopyGetters, Getters};
use std::{cmp::Ordering,
          ffi::CStr,
          path::{Path, PathBuf}};


/// Holds access information for the file and provides access to it.
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct XdrkFile {
  #[getset(get = "pub")]
  path: PathBuf,
  #[getset(get_copy = "pub")]
  idx:  usize,
}

// DESTRUCTOR - CLOSES FILE ------------------------------------------------ //
impl Drop for XdrkFile {
  /// Close the drk/xrk file on `XdrkFile` destruction
  fn drop(&mut self) {
    unsafe { aim::close_file_i(self.idx as i32) };
  }
}

impl XdrkFile {
  // META FUNCTIONS -------------------------------------------------------- //
  /// Library compilation date.
  pub fn library_date() -> Result<NaiveDate> {
    Ok(NaiveDate::parse_from_str(unsafe {
                                   CStr::from_ptr(aim::get_library_date())
                                 }.to_str()?,
                                 "%b %d %Y")?)
  }

  /// Library compilation time.
  pub fn library_time() -> Result<NaiveTime> {
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
  /// Loads a drk/xrk file and creates an `XrdkFile` object.
  pub fn load(path: &Path) -> Result<Self> {
    let extension =
      path.extension()
          .unwrap_or_default()
          .to_str()
          .ok_or(anyhow!("file extension is not valid unicode"))?;

    ensure!(path.exists() && path.is_file(),
            "path does not exist or is not a valid file");
    ensure!(["drk", "xrk"].contains(&extension),
            "only files with extensions .xrk and .drk accepted");

    let idx = unsafe { aim::open_file(srv::path_to_cstring(path)?.as_ptr()) };
    match idx.cmp(&0) {
      Ordering::Greater => Ok(Self { path: path.to_owned(),
                                     idx:  idx as usize, }),
      Ordering::Equal => bail!("file is open but can't be parsed"),
      Ordering::Less => bail!("an error occurred"),
    }
  }

  /// Close the drk/xrk file by path.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION. `XdrkFile` IMPLEMENTS THE `Drop`
  /// TRAIT TO CLOSE FILES, I.E. FILES ARE CLOSED WHEN THE `XdrkFile` OBJECT
  /// GOES OUT OF SCOPE.
  #[doc(hidden)]
  pub fn close_by_path(&self, path: &Path) -> Result<()> {
    ensure!(path == self.path,
            "file '{}' is not associated file",
            path.display());

    let path = srv::path_to_cstring(path)?;
    let ret = unsafe { aim::close_file_n(path.as_ptr()) };
    ensure!(ret == self.idx as i32,
            "file '{}' could not be closed",
            path.to_str()?);

    Ok(())
  }

  /// Close the drk/xrk file by index.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION. `XdrkFile` IMPLEMENTS THE `Drop`
  /// TRAIT TO CLOSE FILES, I.E. FILES ARE CLOSED WHEN THE `XdrkFile` OBJECT
  /// GOES OUT OF SCOPE.
  #[doc(hidden)]
  pub fn close_by_index(&self, idx: i32) -> Result<()> {
    ensure!(idx == self.idx as i32,
            "file '{}' is not associated file",
            idx);

    let ret = unsafe { aim::close_file_i(idx) };
    ensure!(ret == self.idx as i32, "file '{}' could not be closed", idx);

    Ok(())
  }

  // RUN LEVEL FUNCTIONS --------------------------------------------------- //
  pub fn championship(&self) -> Result<String> {
    srv::strptr_to_string(unsafe {
      aim::get_championship_name(self.idx as i32)
    })
  }

  pub fn track(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_track_name(self.idx as i32) })
  }

  pub fn venue_type(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_venue_type_name(self.idx as i32) })
  }

  pub fn vehicle(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_vehicle_name(self.idx as i32) })
  }

  pub fn racer(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_racer_name(self.idx as i32) })
  }

  /// On success, the `Result` contains a datetime object which defines when
  /// this `XdrkFile` was recorded.
  pub fn datetime(&self) -> Result<NaiveDateTime> {
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

  /// On success, the `Result` contains the number of laps in this `XdrkFile`.
  pub fn number_of_laps(&self) -> Result<usize> {
    let count = unsafe { aim::get_laps_count(self.idx as i32) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("file contains 0 laps"),
      Ordering::Less => bail!("error getting lap count"),
    }
  }

  /// For lap with index `lap_idx`, request `LapInfo`. Returns an error if
  /// `lap_idx` is out of range (i.e. the `XdrkFile` does not contain a lap
  /// with that index) or the library calls fails for any reason.
  ///
  /// `LapInfo` objects contain the lap number, the start of the lap within the
  /// run recorded in this file (via the `start()` getter) and the lap duration
  /// (via the `duration()` getter).
  pub fn lap_info(&self, lap_idx: usize) -> Result<LapInfo> {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");

    let (mut start, mut time) = (0.0f64, 0.0f64);
    let err_code = unsafe {
      aim::get_lap_info(self.idx as i32, lap_idx as i32, &mut start, &mut time)
    };
    ensure!(err_code == 1, "could not fetch lap info");

    Ok(LapInfo::new(lap_idx, start, time))
  }

  /// For lap with index `idx`, request all channels. Returns a Lap object or
  /// an error if `idx` is out of range (i.e. the `XdrkFile` does not contain a
  /// lap with that index) or the library call fails for any reason.
  ///
  /// `Lap` objects contain a `LapInfo` object and a `Vec<Channel>` containing
  /// all data recorded in the lap.
  pub fn lap(&self, lap_idx: usize) -> Result<Lap> {
    let mut raw_channels = Vec::with_capacity(self.number_of_channels()?);
    for channel_name in self.channel_names()? {
      raw_channels.push(self.raw_channel(&channel_name, Some(lap_idx))?);
    }
    Ok(Lap::from_raw_channels(self.lap_info(lap_idx)?, raw_channels))
  }

  /// Request all channels for all laps contained in this `XdrkFile`. Fails if
  /// the library call fails for any reason, either on finding all laps or on
  /// requesting `Lap`s.
  pub fn all_laps(&self) -> Result<Vec<Lap>> {
    let mut laps = Vec::with_capacity(self.number_of_laps()?);
    for lap_idx in 0..self.number_of_laps()? {
      laps.push(self.lap(lap_idx)?);
    }
    Ok(laps)
  }

  /// On success, the `Result` contains the total number of channels in this
  /// `XdrkFile`, including the GPS channels (but not the raw GPS channels).
  pub fn number_of_channels(&self) -> Result<usize> {
    Ok(self.channels_count()? + self.gps_channels_count()?)
  }

  /// Request a list of all channel names which occur in this `XdrkFile`. Fails
  /// if the library call fails for any reason, either on finding all channels
  /// of on requesting channel names. INCLUDES GPS CHANNELS.
  pub fn channel_names(&self) -> Result<Vec<String>> {
    let mut names = Vec::with_capacity(self.number_of_channels()?);

    for idx in 0..self.channels_count()? {
      names.push(self.channel_name(idx)?);
    }
    for idx in 0..self.gps_channels_count()? {
      names.push(self.gps_channel_name(idx)?);
    }

    Ok(names)
  }

  /// For channel with index `idx`, request the channel name.
  pub fn channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.number_of_channels()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      if channel_idx < self.channels_count()? {
        aim::get_channel_name(self.idx as i32, channel_idx as i32)
      } else {
        let channel_idx = channel_idx % self.channels_count()?;
        aim::get_GPS_channel_name(self.idx as i32, channel_idx as i32)
      }
    })
  }

  /// Request index of channel with name `channel_name`.
  pub fn channel_idx(&self, channel_name: &str) -> Result<usize> {
    let channel_idx =
      self.channel_names()?
          .iter()
          .position(|name| name == channel_name)
          .ok_or(anyhow!("no channel '{}' found", channel_name))?;

    assert!(channel_idx < self.number_of_channels()?,
            "channel index out of range");
    Ok(channel_idx)
  }

  /// For channel with name `channel_name`, request the channel unit.
  pub fn channel_unit(&self, channel_name: &str) -> Result<String> {
    let channel_idx = self.channel_idx(channel_name)?;
    srv::strptr_to_string(unsafe {
      if channel_idx < self.channels_count()? {
        aim::get_channel_units(self.idx as i32, channel_idx as i32)
      } else {
        let channel_idx = channel_idx % self.channels_count()?;
        aim::get_GPS_channel_units(self.idx as i32, channel_idx as i32)
      }
    })
  }

  /// Request a `RawChannel` object by name and lap index. Fails if no channel
  /// with that name exists, if no lap with that index exists or the library
  /// call fails for any reason. Pass `None` for lap to get the raw channel
  /// with data from all laps.
  pub fn raw_channel(&self,
                     channel_name: &str,
                     lap_idx: Option<usize>)
                     -> Result<RawChannel>
  {
    Ok(RawChannel::new(channel_name.to_owned(),
                       self.channel_unit(channel_name)?,
                       self.raw_channel_data(channel_name, lap_idx)?))
  }

  /// For channel with name `channel_name`, collect the measurement samples in
  /// a `RawChannelData` object. Data is unsynchronized. GPS data included.
  pub fn raw_channel_data(&self,
                          channel_name: &str,
                          lap_idx: Option<usize>)
                          -> Result<RawChannelData>
  {
    let channel_idx = self.channel_idx(channel_name)?;

    if let Some(lap_idx) = lap_idx {
      if channel_idx < self.channels_count()? {
        self.lap_channel_samples(lap_idx, channel_idx)
      } else {
        let channel_idx = channel_idx % self.channels_count()?;
        self.lap_gps_channel_samples(lap_idx, channel_idx)
      }
    } else {
      if channel_idx < self.channels_count()? {
        self.channel_samples(channel_idx)
      } else {
        let channel_idx = channel_idx % self.channels_count()?;
        self.gps_channel_samples(channel_idx)
      }
    }
  }

  // ----------------------------------------------------------------------- //
  // RAW LIBRARY FUNCTIONS ------------------------------------------------- //
  // ----------------------------------------------------------------------- //
  /// On success, the `Result` contains the number of channels in this
  /// `XdrkFile`.
  pub fn channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_channels_count(self.idx as i32) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("file contains 0 channels"),
      Ordering::Less => bail!("error getting channel count"),
    }
  }

  /// For channel with index `channel_idx`, request the number of samples
  /// contained in this `XdrkFile`.
  pub fn channel_samples_count(&self, channel_idx: usize) -> Result<usize> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

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
  /// this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn channel_samples(&self, channel_idx: usize) -> Result<RawChannelData> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count = self.channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_channel_samples(self.idx as i32,
                               channel_idx as i32,
                               timestamps.as_mut_ptr(),
                               samples.as_mut_ptr(),
                               count as i32)
    };
    ensure!(read == count as i32, "error reading channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and channel with index `channel_idx`,
  /// request the number of samples contained in this `XdrkFile`.
  pub fn lap_channel_samples_count(&self,
                                   lap_idx: usize,
                                   channel_idx: usize)
                                   -> Result<usize>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

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
  /// request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_channel_samples(&self,
                             lap_idx: usize,
                             channel_idx: usize)
                             -> Result<RawChannelData>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count = self.lap_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_channel_samples(self.idx as i32,
                                   lap_idx as i32,
                                   channel_idx as i32,
                                   timestamps.as_mut_ptr(),
                                   samples.as_mut_ptr(),
                                   count as i32)
    };
    ensure!(read == count as i32, "error reading lap channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
  }

  // GPS INFORMATION FUNCTIONS --------------------------------------------- //
  //
  // GPS channels are the same channels added to AiM drk files in RS2Analysis,
  // those that consider vehicle dynamics assuming that the vehicle is
  // constantly aligned to the trajectory.
  //
  /// On success, the `Result` contains the number of GPS channels in this
  /// `XdrkFile`.
  pub fn gps_channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_GPS_channels_count(self.idx as i32) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("file contains 0 GPS channels"),
      Ordering::Less => bail!("error getting GPS channel count"),
    }
  }

  /// For GPS channel with index `channel_idx`, request the channel name.
  pub fn gps_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_channel_name(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS channel with index `channel_idx`, request the GPS channel unit.
  pub fn gps_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_channel_units(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS channel with index `channel_idx`, request the number of samples
  /// contained in this `XdrkFile`.
  pub fn gps_channel_samples_count(&self,
                                   channel_idx: usize)
                                   -> Result<usize>
  {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

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
  /// in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn gps_channel_samples(&self,
                             channel_idx: usize)
                             -> Result<RawChannelData>
  {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    let count = self.gps_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_GPS_channel_samples(self.idx as i32,
                                   channel_idx as i32,
                                   timestamps.as_mut_ptr(),
                                   samples.as_mut_ptr(),
                                   count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and GPS channel with index `channel_idx`,
  /// request the number of samples contained in this `XdrkFile`.
  pub fn lap_gps_channel_samples_count(&self,
                                       lap_idx: usize,
                                       channel_idx: usize)
                                       -> Result<usize>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

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
  /// request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_gps_channel_samples(&self,
                                 lap_idx: usize,
                                 channel_idx: usize)
                                 -> Result<RawChannelData>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    let count = self.lap_gps_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_GPS_channel_samples(self.idx as i32,
                                       lap_idx as i32,
                                       channel_idx as i32,
                                       timestamps.as_mut_ptr(),
                                       samples.as_mut_ptr(),
                                       count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
  }

  // ----------------------------------------------------------------------- //
  // RAW GPS FUNCTIONS ----------------------------------------------------- //
  // ----------------------------------------------------------------------- //
  /// On success, the `Result` contains the number of GPS raw channels in this
  /// `XdrkFile`.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_GPS_raw_channels_count(self.idx as i32) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => bail!("file contains 0 GPS channels"),
      Ordering::Less => bail!("error getting GPS channel count"),
    }
  }

  /// For GPS raw channel with index `channel_idx`, request the channel name.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_name(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the GPS channel
  /// unit.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_units(self.idx as i32, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the number of
  /// samples contained in this `XdrkFile`.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_samples_count(&self,
                                       channel_idx: usize)
                                       -> Result<usize>
  {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

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
  /// contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn gps_raw_channel_samples(&self,
                                 channel_idx: usize)
                                 -> Result<RawChannelData>
  {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = self.gps_raw_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_GPS_raw_channel_samples(self.idx as i32,
                                       channel_idx as i32,
                                       timestamps.as_mut_ptr(),
                                       samples.as_mut_ptr(),
                                       count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and GPS raw channel with index
  /// `channel_idx`, request the number of samples contained in this
  /// `XdrkFile`.
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn lap_gps_raw_channel_samples_count(&self,
                                           lap_idx: usize,
                                           channel_idx: usize)
                                           -> Result<usize>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

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
  /// `channel_idx`, request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `RawChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION.
  pub fn lap_gps_raw_channel_samples(&self,
                                     lap_idx: usize,
                                     channel_idx: usize)
                                     -> Result<RawChannelData>
  {
    ensure!(lap_idx < self.number_of_laps()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = self.lap_gps_raw_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = RawChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_GPS_raw_channel_samples(self.idx as i32,
                                           lap_idx as i32,
                                           channel_idx as i32,
                                           timestamps.as_mut_ptr(),
                                           samples.as_mut_ptr(),
                                           count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(RawChannelData::from_tsc(timestamps, samples, count))
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
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn xdrkfile_test() {
    // DESTRUCTOR TEST
    // opening XRK and DRK files produces temporary files which are cleaned
    // up when the file is closed, which we do via `Drop` so it happens when
    // the object goes out of scope. to test this is working, we wrap the
    // actual test in a block...
    {
      let xrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();
      assert!(xrk_file.idx() > 0);
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
    // END DESTRUCTOR TEST

    let xrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();

    assert_eq!("AU-RS3-R5-S-S", &xrk_file.vehicle().unwrap());
    assert_eq!("ARA_1-0-0", &xrk_file.track().unwrap());
    assert_eq!("017", &xrk_file.racer().unwrap());
    assert_eq!("WT-20", &xrk_file.championship().unwrap());
    assert_eq!("Q3", &xrk_file.venue_type().unwrap());
    assert_eq!(NaiveDate::from_ymd(2020, 11, 14).and_hms(16, 49, 39),
               xrk_file.datetime().unwrap());

    assert_eq!(4, xrk_file.number_of_laps().unwrap());
    assert_eq!(LapInfo::new(2, 383.258, 170.488),
               xrk_file.lap_info(2).unwrap());

    assert_eq!(51, xrk_file.number_of_channels().unwrap());

    macro_rules! stringvec {
      ($($x:literal),* $(,)?) => (vec![$($x.to_string()),*]);
    }
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
                                   "mEngTorq",
                                   "mEngTorqTarget",
                                   "posGearDSG",
                                   "swGearUP",
                                   "swGearDOWN",
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
    assert_eq!(channel_names, xrk_file.channel_names().unwrap());

    assert_eq!("Logger Temperature", &xrk_file.channel_name(0).unwrap());
    assert_eq!("pManifoldScrut", &xrk_file.channel_name(2).unwrap());
    assert_eq!("fEngRpm", &xrk_file.channel_name(15).unwrap());
    assert_eq!("GPS Speed", &xrk_file.channel_name(40).unwrap());
    assert_eq!("GPS Nsat", &xrk_file.channel_name(41).unwrap());

    assert_eq!("C", &xrk_file.channel_unit("Logger Temperature").unwrap());
    assert_eq!("bar", &xrk_file.channel_unit("pManifoldScrut").unwrap());
    assert_eq!("rpm", &xrk_file.channel_unit("fEngRpm").unwrap());
    assert_eq!("m/s", &xrk_file.channel_unit("GPS Speed").unwrap());
    assert_eq!("#", &xrk_file.channel_unit("GPS Nsat").unwrap());

    assert_eq!(553, xrk_file.channel_samples_count(0).unwrap());
    assert_eq!(57980, xrk_file.channel_samples_count(2).unwrap());
    assert_eq!(57952, xrk_file.channel_samples_count(15).unwrap());
    assert_eq!(58006, xrk_file.gps_channel_samples_count(0).unwrap());
    assert_eq!(58006, xrk_file.gps_channel_samples_count(1).unwrap());

    assert_eq!(false, xrk_file.channel_samples(0).unwrap().is_empty());
    assert_eq!(162, xrk_file.lap_channel_samples_count(2, 0).unwrap());
    assert_eq!(false,
               xrk_file.lap_channel_samples(2, 0).unwrap().is_empty());
  }

  #[test]
  fn meta_fn() {
    let (date, time) = {
      #[cfg(target_family = "unix")]
      let date = NaiveDate::from_ymd(2020, 1, 24);
      #[cfg(target_family = "windows")]
      let date = NaiveDate::from_ymd(2018, 8, 1);

      #[cfg(target_family = "unix")]
      let time = NaiveTime::from_hms(16, 36, 19);
      #[cfg(target_family = "windows")]
      let time = NaiveTime::from_hms(12, 22, 53);

      (date, time)
    };

    assert_eq!(date.and_time(time), XdrkFile::library_datetime().unwrap());
    assert_eq!(date, XdrkFile::library_date().unwrap());
    assert_eq!(time, XdrkFile::library_time().unwrap());
  }
}
