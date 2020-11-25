// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use super::{ensure,
            fubar,
            fubar::Result,
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

    match 0.cmp(&idx) {
      Ordering::Less => Ok(Self { path, idx }),
      Ordering::Equal => fubar!("file is open but can't be parsed"),
      Ordering::Greater => fubar!("an error occurred"),
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
  ///
  /// # Returns
  /// an empty `Ok()` on success.
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
  /// # Return
  /// - a `NaiveDateTime` object indicating when this file was produced
  pub fn date_time(&self) -> Result<NaiveDateTime> {
    let tm = unsafe { *aim::get_date_and_time(self.idx) };
    Ok(NaiveDate::from_ymd(tm.tm_year + 1900,
                           (tm.tm_mon + 1) as u32,
                           tm.tm_mday as u32).and_hms(tm.tm_hour as u32,
                                                      tm.tm_min as u32,
                                                      tm.tm_sec as u32))
  }
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
