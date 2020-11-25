// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use super::{ensure,
            fubar,
            fubar::Result,
            storage::LapInfo,
            service as srv,
            xdrkbindings as aim};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use getset::{CopyGetters, Getters};
use std::{cmp::Ordering,
          ffi::CStr,
          path::{Path, PathBuf}};


#[derive(Debug, CopyGetters, Getters)]
pub struct XdrkFile {
  #[getset(get = "pub")]
  path: PathBuf,
  #[getset(get_copy = "pub")]
  idx:  i32,
}


// FILE OPENING / CLOSING FUNCTIONS ---------------------------------------- //
impl XdrkFile {
  /// Open a drk/xrk file
  ///
  /// # Arguments
  /// - `path`: full path to the file to be opened
  ///
  /// # Returns
  /// a valid `XdrkFile` object.
  pub fn load(path: &Path) -> Result<Self> {
    ensure!(path.exists()
            && path.is_file()
            && path.extension().unwrap_or_default() == "xrk",
            "path does not exist or not a file");

    let path = path.to_owned();
    let idx = unsafe { aim::open_file(srv::path_to_cstring(&path)?.as_ptr()) };

    match idx.cmp(&0) {
      Ordering::Greater => Ok(Self { path, idx }),
      Ordering::Equal => fubar!("file is open but can't be parsed"),
      Ordering::Less => fubar!("an error occurred"),
    }
  }

  /// Close the drk/xrk file by path
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION. `XdrkFile` IMPLEMENTS THE `Drop`
  /// TRAIT TO CLOSE FILES, I.E. FILES ARE CLOSED WHEN THE `XdrkFile` OBJECT
  /// GOES OUT OF SCOPE.
  ///
  /// # Arguments
  /// - `path`: full path to the file to be closed
  ///
  /// # Returns
  /// an empty `Ok()` on success.
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

  /// Close the drk/xrk file by index
  ///
  /// THIS SHOULD NEVER BE USED DIRECTLY AND IS ONLY PROVIDED AS AN INTERFACE
  /// TO THE UNDERLYING LIBRARY FUNCTION. `XdrkFile` IMPLEMENTS THE `Drop`
  /// TRAIT TO CLOSE FILES, I.E. FILES ARE CLOSED WHEN THE `XdrkFile` OBJECT
  /// GOES OUT OF SCOPE.
  ///
  /// # Arguments
  /// - `idx`: index to the file to be closed
  ///
  /// # Returns
  /// an empty `Ok()` on success.
  pub fn close_by_index(&self, idx: i32) -> Result<()> {
    ensure!(idx == self.idx, "file '{}' is not associated file", idx);

    let ret = unsafe { aim::close_file_i(idx) };
    ensure!(ret == self.idx, "file '{}' could not be closed", idx);

    Ok(())
  }
}

// DESTRUCTOR - CLOSES FILE ------------------------------------------------ //
impl Drop for XdrkFile {
  /// Close the drk/xrk file on `XdrkFile` destruction
  fn drop(&mut self) {
    unsafe { aim::close_file_i(self.idx) };
  }
}

// SESSION INFORMATION FUNCTIONS ------------------------------------------- //
impl XdrkFile {
  /// Get vehicle info
  ///
  /// # Returns
  /// - a `String` containing vehicle info
  pub fn vehicle_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_vehicle_name(self.idx) })
  }

  /// Get track info
  ///
  /// # Returns
  /// - a `String` containing track info
  pub fn track_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_track_name(self.idx) })
  }

  /// Get racer info
  ///
  /// # Returns
  /// - a `String` containing racer info
  pub fn racer_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_racer_name(self.idx) })
  }

  /// Get championship info
  ///
  /// # Returns
  /// - a `String` containing championship info
  pub fn championship_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_championship_name(self.idx) })
  }

  /// Get venue type info
  ///
  /// # Returns
  /// - a `String` containing venue info
  pub fn venue_type_name(&self) -> Result<String> {
    srv::strptr_to_string(unsafe { aim::get_venue_type_name(self.idx) })
  }

  /// Get session date and time
  ///
  /// # Returns
  /// - a `NaiveDateTime` object indicating when this file was produced
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

  /// Get number of laps contained in drk/xrk file
  ///
  /// # Returns
  /// - the number of laps
  pub fn laps_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_laps_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => fubar!("file contains 0 laps"),
      Ordering::Less => fubar!("error getting lap count"),
    }
  }

  /// Get lap info
  ///
  /// # Arguments
  /// - `lap_idx`: index of the lap in question
  ///
  /// # Returns
  /// - a `LapInfo` object, which contains
  /// - start time since start of session in seconds
  /// - duration, a.k.a laptime
  pub fn lap_info(&self, lap_idx: usize) -> Result<LapInfo> {
    let (mut start, mut duration) = (0.0f64, 0.0f64);

    let err_code = unsafe {
      aim::get_lap_info(self.idx, lap_idx as i32, &mut start, &mut duration)
    };
    ensure!(err_code == 1, "could not fetch lap info");

    Ok(LapInfo::new(start, duration))
  }
}

// CHANNEL INFORMATION FUNCTIONS ------------------------------------------- //
impl XdrkFile {
  /// Get number of channels contained in a drk/xrk file
  ///
  /// # Returns
  /// - the number of channels
  pub fn channels_count(&self) -> Result<usize> {
    let count = unsafe { aim::get_channels_count(self.idx) };
    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => fubar!("file contains 0 channels"),
      Ordering::Less => fubar!("error getting channel count"),
    }
  }

  /// Get channel name
  ///
  /// # Arguments
  /// - `channel_idx`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel name
  /// - on error, `NULL`
  pub fn channel_name(&self, channel_idx: usize) -> Result<String> {
    srv::strptr_to_string(unsafe {
      aim::get_channel_name(self.idx, channel_idx as i32)
    })
  }

  /// Get channel units
  ///
  /// # Arguments
  /// - `channel_idx`: the channel index
  ///
  /// # Returns
  /// - a `String` containing channel units
  pub fn channel_units(&self, channel_idx: usize) -> Result<String> {
    srv::strptr_to_string(unsafe {
      aim::get_channel_units(self.idx, channel_idx as i32)
    })
  }

  /// Get number of datapoints in channel
  ///
  /// # Arguments
  /// - `channel_idx`: the channel index
  ///
  /// # Returns
  /// - the number of datapoints in the channel
  ///
  /// - on success, the number of datapoints in the channel
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn channel_samples_count(&self, channel_idx: usize) -> Result<usize> {
    let count = unsafe {
      aim::get_channel_samples_count(self.idx, channel_idx as i32)
    };

    match count.cmp(&0) {
      Ordering::Greater => Ok(count as usize),
      Ordering::Equal => fubar!("channel contains 0 samples"),
      Ordering::Less => fubar!("error getting channel samples count"),
    }
  }

  /*
  /// Get datapoints in channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_channel_samples(idxf: c_int,
                             idxc: c_int,
                             ptimes: *mut f64,
                             pvalues: *mut f64,
                             cnt: c_int)
                             -> c_int;

  /// Get number of datapoints in channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel in the lap
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_channel_samples_count(idxf: c_int,
                                       idxl: c_int,
                                       idxc: c_int)
                                       -> c_int;

  /// Get datapoints in channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_channel_samples(idxf: c_int,
                                 idxl: c_int,
                                 idxc: c_int,
                                 ptimes: *mut f64,
                                 pvalues: *mut f64,
                                 cnt: c_int)
                                 -> c_int;
  */
}

// META FUNCTIONS ---------------------------------------------------------- //
impl XdrkFile {
  /// Library compilation date
  ///
  /// # Returns
  /// - the compile date of this library
  pub fn library_date() -> Result<NaiveDate> {
    Ok(NaiveDate::parse_from_str(unsafe {
                                   CStr::from_ptr(aim::get_library_date())
                                 }.to_str()?,
                                 "%b %d %Y")?)
  }

  /// Library compilation time
  ///
  /// # Returns
  /// - the compile time of this library
  pub fn library_time() -> Result<NaiveTime> {
    Ok(NaiveTime::parse_from_str(unsafe {
                                   CStr::from_ptr(aim::get_library_time())
                                 }.to_str()?,
                                 "%H:%M:%S")?)
  }

  /// Library compilation date and time
  ///
  /// This is a convenience function wrapping the functions `library_date` and
  /// `library_time` to produce a datetime object.
  ///
  /// # Returns
  /// - the compile datetime of this library
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
    assert_eq!(LapInfo::new(383.258, 170.488), xrk_file.lap_info(2).unwrap());

    // CHANNEL INFORMATION FUNCTIONS --------------------------------------- //
    assert_eq!(40, xrk_file.channels_count().unwrap());

    assert_eq!("Logger Temperature", &xrk_file.channel_name(0).unwrap());
    assert_eq!("pManifoldScrut", &xrk_file.channel_name(2).unwrap());
    assert_eq!("fEngRpm", &xrk_file.channel_name(15).unwrap());

    assert_eq!("C", &xrk_file.channel_units(0).unwrap());
    assert_eq!("bar", &xrk_file.channel_units(2).unwrap());
    assert_eq!("rpm", &xrk_file.channel_units(15).unwrap());

    assert_eq!(553, xrk_file.channel_samples_count(0).unwrap());
    assert_eq!(57980, xrk_file.channel_samples_count(2).unwrap());
    assert_eq!(57952, xrk_file.channel_samples_count(15).unwrap());

    for i in 0..40 {
      println!("{:#?}", xrk_file.channel_samples_count(i).unwrap());
    }
  }

  #[test]
  fn meta_fn() {
    let date = NaiveDate::from_ymd(2020, 1, 24);
    let time = NaiveTime::from_hms(16, 36, 19);

    assert_eq!(date.and_time(time), XdrkFile::library_datetime().unwrap());
    assert_eq!(date, XdrkFile::library_date().unwrap());
    assert_eq!(time, XdrkFile::library_time().unwrap());
  }
}
