// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use super::{service as srv,
            storage::{ChannelData, LapInfo},
            xdrkbindings as aim};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use fubar::{decamp, ensure, Result};
use getset::{CopyGetters, Getters};
use std::{cmp::Ordering,
          ffi::CStr,
          path::{Path, PathBuf}};


/// Holds access information for the file and provides access to it.
#[derive(Debug, CopyGetters, Getters)]
pub struct XdrkFile {
  #[getset(get = "pub")]
  path: PathBuf,
  #[getset(get_copy = "pub")]
  idx:  i32,
}

// DESTRUCTOR - CLOSES FILE ------------------------------------------------ //
impl Drop for XdrkFile {
  /// Close the drk/xrk file on `XdrkFile` destruction
  fn drop(&mut self) {
    unsafe { aim::close_file_i(self.idx) };
  }
}

impl XdrkFile {
  // FILE OPENING / CLOSING FUNCTIONS -------------------------------------- //
  /// Loads a drk/xrk file and creates an `XrdkFile` object.
  pub fn load(path: &Path) -> Result<Self> {
    ensure!(path.exists()
            && path.is_file()
            && path.extension().unwrap_or_default() == "xrk",
            "path does not exist or not a valid file");

    let path = path.to_owned();
    let idx = unsafe { aim::open_file(srv::path_to_cstring(&path)?.as_ptr()) };

    match idx.cmp(&0) {
      Ordering::Greater => Ok(Self { path, idx }),
      Ordering::Equal => decamp!("file is open but can't be parsed"),
      Ordering::Less => decamp!("an error occurred"),
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
    ensure!(ret == self.idx,
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
    ensure!(idx == self.idx, "file '{}' is not associated file", idx);

    let ret = unsafe { aim::close_file_i(idx) };
    ensure!(ret == self.idx, "file '{}' could not be closed", idx);

    Ok(())
  }

  // SESSION INFORMATION FUNCTIONS ----------------------------------------- //
  pub fn vehicle_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_vehicle_name(self.idx) })
  }

  pub fn track_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_track_name(self.idx) })
  }

  pub fn racer_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_racer_name(self.idx) })
  }

  pub fn championship_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_championship_name(self.idx) })
  }

  pub fn venue_type_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_venue_type_name(self.idx) })
  }

  /// On success, the `Result` contains a datetime object which defines when
  /// this `XdrkFile` was recorded.
  pub fn date_time(&self) -> Result<NaiveDateTime> {
    let tm: *const aim::tm = unsafe { aim::get_date_and_time(self.idx) };
    ensure!(!tm.is_null(), "could not fetch datetime object");

    let tm = unsafe { *tm };
    Ok(NaiveDate::from_ymd(tm.tm_year + 1900,
                           (tm.tm_mon + 1) as u32,
                           tm.tm_mday as u32).and_hms(tm.tm_hour as u32,
                                                      tm.tm_min as u32,
                                                      tm.tm_sec as u32))
  }

  /// On success, the `Result` contains the number of laps in this `XdrkFile`.
  pub fn laps_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_laps_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("file contains 0 laps"),
      Ordering::Less => decamp!("error getting lap count"),
    }
  }

  /// For lap with index `lap_idx`, request `LapInfo`. Returns an error if
  /// `lap_idx` is out of range (i.e. the `XdrkFile` does not contain a lap
  /// with that index) or the library calls fails for any reason.
  ///
  /// `LapInfo` objects contain the start of the lap within the run recorded in
  /// this file (via the `start()` getter) and the lap duration (via the
  /// `duration()` getter).
  pub fn lap_info(&self, lap_idx: usize) -> Result<LapInfo> {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");

    let (mut start, mut duration) = (0.0f64, 0.0f64);
    let err_code = unsafe {
      aim::get_lap_info(self.idx, lap_idx as i32, &mut start, &mut duration)
    };
    ensure!(err_code == 1, "could not fetch lap info");

    Ok(LapInfo::new(start, duration))
  }

  // CHANNEL INFORMATION FUNCTIONS ----------------------------------------- //
  /// On success, the `Result` contains the number of channels in this
  /// `XdrkFile`.
  pub fn channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_channels_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("file contains 0 channels"),
      Ordering::Less => decamp!("error getting channel count"),
    }
  }

  /// For channel with index `channel_idx`, request the channel name.
  pub fn channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_channel_name(self.idx, channel_idx as i32)
    })
  }

  /// For channel with index `channel_idx`, request the channel unit.
  pub fn channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_channel_units(self.idx, channel_idx as i32)
    })
  }

  /// For channel with index `channel_idx`, request the number of samples
  /// contained in this `XdrkFile`.
  pub fn channel_samples_count(&self, channel_idx: usize) -> Result<usize> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count =
      unsafe { aim::get_channel_samples_count(self.idx, channel_idx as i32) };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("channel contains 0 samples"),
      Ordering::Less => decamp!("error getting channel samples count"),
    }
  }

  /// For channel with index `channel_idx`, request the samples contained in
  /// this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn channel_samples(&self, channel_idx: usize) -> Result<ChannelData> {
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count = self.channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_channel_samples(self.idx,
                               channel_idx as i32,
                               timestamps.as_mut_ptr(),
                               samples.as_mut_ptr(),
                               count as i32)
    };
    ensure!(read == count as i32, "error reading channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and channel with index `channel_idx`,
  /// request the number of samples contained in this `XdrkFile`.
  pub fn lap_channel_samples_count(&self,
                                   lap_idx: usize,
                                   channel_idx: usize)
                                   -> Result<usize>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count = unsafe {
      aim::get_lap_channel_samples_count(self.idx,
                                         lap_idx as i32,
                                         channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("channel contains 0 samples in this lap"),
      Ordering::Less => decamp!("error getting lap channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and channel with index `channel_idx`,
  /// request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_channel_samples(&self,
                             lap_idx: usize,
                             channel_idx: usize)
                             -> Result<ChannelData>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.channels_count()?,
            "channel_idx out of range");

    let count = self.lap_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_channel_samples(self.idx,
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
  /// On success, the `Result` contains the number of GPS channels in this
  /// `XdrkFile`.
  pub fn gps_channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_GPS_channels_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("file contains 0 GPS channels"),
      Ordering::Less => decamp!("error getting GPS channel count"),
    }
  }

  /// For GPS channel with index `channel_idx`, request the channel name.
  pub fn gps_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_channel_name(self.idx, channel_idx as i32)
    })
  }

  /// For GPS channel with index `channel_idx`, request the GPS channel unit.
  pub fn gps_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_channel_units(self.idx, channel_idx as i32)
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
      aim::get_GPS_channel_samples_count(self.idx, channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("GPS channel contains 0 samples"),
      Ordering::Less => decamp!("error getting GPS channel samples count"),
    }
  }

  /// For GPS channel with index `channel_idx`, request the samples contained
  /// in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn gps_channel_samples(&self,
                             channel_idx: usize)
                             -> Result<ChannelData>
  {
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    let count = self.gps_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_GPS_channel_samples(self.idx,
                                   channel_idx as i32,
                                   timestamps.as_mut_ptr(),
                                   samples.as_mut_ptr(),
                                   count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// For lap with index `lap_idx` and GPS channel with index `channel_idx`,
  /// request the number of samples contained in this `XdrkFile`.
  pub fn lap_gps_channel_samples_count(&self,
                                       lap_idx: usize,
                                       channel_idx: usize)
                                       -> Result<usize>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    let count = unsafe {
      aim::get_lap_GPS_channel_samples_count(self.idx,
                                             lap_idx as i32,
                                             channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("GPS channel contains 0 samples in this lap"),
      Ordering::Less => decamp!("error getting lap GPS channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and GPS channel with index `channel_idx`,
  /// request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_gps_channel_samples(&self,
                                 lap_idx: usize,
                                 channel_idx: usize)
                                 -> Result<ChannelData>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_channels_count()?,
            "channel_idx out of range");

    let count = self.lap_gps_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_GPS_channel_samples(self.idx,
                                       lap_idx as i32,
                                       channel_idx as i32,
                                       timestamps.as_mut_ptr(),
                                       samples.as_mut_ptr(),
                                       count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

  /// On success, the `Result` contains the number of GPS raw channels in this
  /// `XdrkFile`.
  pub fn gps_raw_channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_GPS_raw_channels_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("file contains 0 GPS channels"),
      Ordering::Less => decamp!("error getting GPS channel count"),
    }
  }

  /// For GPS raw channel with index `channel_idx`, request the channel name.
  pub fn gps_raw_channel_name(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_name(self.idx, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the GPS channel
  /// unit.
  pub fn gps_raw_channel_unit(&self, channel_idx: usize) -> Result<String> {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    srv::strptr_to_string(unsafe {
      aim::get_GPS_raw_channel_units(self.idx, channel_idx as i32)
    })
  }

  /// For GPS raw channel with index `channel_idx`, request the number of
  /// samples contained in this `XdrkFile`.
  pub fn gps_raw_channel_samples_count(&self,
                                       channel_idx: usize)
                                       -> Result<usize>
  {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = unsafe {
      aim::get_GPS_raw_channel_samples_count(self.idx, channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("GPS channel contains 0 samples"),
      Ordering::Less => decamp!("error getting GPS channel samples count"),
    }
  }

  /// For GPS raw channel with index `channel_idx`, request the samples
  /// contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn gps_raw_channel_samples(&self,
                                 channel_idx: usize)
                                 -> Result<ChannelData>
  {
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = self.gps_raw_channel_samples_count(channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_GPS_raw_channel_samples(self.idx,
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
  /// `XdrkFile`.
  pub fn lap_gps_raw_channel_samples_count(&self,
                                           lap_idx: usize,
                                           channel_idx: usize)
                                           -> Result<usize>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = unsafe {
      aim::get_lap_GPS_raw_channel_samples_count(self.idx,
                                                 lap_idx as i32,
                                                 channel_idx as i32)
    };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => decamp!("GPS channel contains 0 samples in this lap"),
      Ordering::Less => decamp!("error getting lap GPS channel samples count"),
    }
  }

  /// For lap with index `lap_idx` and GPS raw channel with index
  /// `channel_idx`, request the samples contained in this `XdrkFile`.
  ///
  /// The data will be returned in the form of a `ChannelData` object, which
  /// contains the data as a set of timestamps (the `timestamps()` getter
  /// returns a `&Vec<f64>`) and a corresponding set of samples (the
  /// `samples()` getter returns another `&Vec<f64>`).
  pub fn lap_gps_raw_channel_samples(&self,
                                     lap_idx: usize,
                                     channel_idx: usize)
                                     -> Result<ChannelData>
  {
    ensure!(lap_idx < self.laps_count()?, "lap_idx out of range");
    ensure!(channel_idx < self.gps_raw_channels_count()?,
            "channel_idx out of range");

    let count = self.lap_gps_raw_channel_samples_count(lap_idx, channel_idx)?;
    let (mut timestamps, mut samples) = ChannelData::allocate(count);
    let read = unsafe {
      aim::get_lap_GPS_raw_channel_samples(self.idx,
                                           lap_idx as i32,
                                           channel_idx as i32,
                                           timestamps.as_mut_ptr(),
                                           samples.as_mut_ptr(),
                                           count as i32)
    };
    ensure!(read == count as i32, "error reading GPS channel samples");

    Ok(ChannelData::from_tsc(timestamps, samples, count))
  }

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
}

// LIBRARY CODE END -------------------------------------------------------- //


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use std::fs;

  static XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn xdrkrs_test() {
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

    let xrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();


    // SESSION INFORMATION FUNCTIONS --------------------------------------- //
    assert_eq!("AU-RS3-R5-S-S", &xrk_file.vehicle_name().unwrap());
    assert_eq!("ARA_1-0-0", &xrk_file.track_name().unwrap());
    assert_eq!("017", &xrk_file.racer_name().unwrap());
    assert_eq!("WT-20", &xrk_file.championship_name().unwrap());
    assert_eq!("Q3", &xrk_file.venue_type_name().unwrap());
    assert_eq!(NaiveDate::from_ymd(2020, 11, 14).and_hms(16, 49, 39),
               xrk_file.date_time().unwrap());

    assert_eq!(4, xrk_file.laps_count().unwrap());
    assert_eq!(LapInfo::new(383.258, 170.488),
               xrk_file.lap_info(2).unwrap());

    // CHANNEL INFORMATION FUNCTIONS --------------------------------------- //
    assert_eq!(40, xrk_file.channels_count().unwrap());

    assert_eq!("Logger Temperature", &xrk_file.channel_name(0).unwrap());
    assert_eq!("pManifoldScrut", &xrk_file.channel_name(2).unwrap());
    assert_eq!("fEngRpm", &xrk_file.channel_name(15).unwrap());

    assert_eq!("C", &xrk_file.channel_unit(0).unwrap());
    assert_eq!("bar", &xrk_file.channel_unit(2).unwrap());
    assert_eq!("rpm", &xrk_file.channel_unit(15).unwrap());

    assert_eq!(553, xrk_file.channel_samples_count(0).unwrap());
    assert_eq!(57980, xrk_file.channel_samples_count(2).unwrap());
    assert_eq!(57952, xrk_file.channel_samples_count(15).unwrap());

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
