// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <jonas@bmc-labs.com>

use anyhow::{anyhow, ensure, Result};
use std::{ffi::{CStr, CString},
          os::raw::c_char,
          path::Path};


/// Converts a `*const c_char`, i.e. a raw C string (`const char *` in C), to a
/// Rust `std::ffi::CString`, which is owned. This guarantees lifetime safety.
pub fn strptr_to_cstring(strptr: *const c_char) -> Result<CString> {
  ensure!(!strptr.is_null(), "error: fetched null pointer");
  Ok(unsafe { CStr::from_ptr(strptr) }.to_owned())
}

/// Convenience function to convert directly to Rust's `String` type from a
/// `*const c_char`, i.e. a raw C string (`const char *` in C).
pub fn strptr_to_string(strptr: *const c_char) -> Result<String> {
  Ok(strptr_to_cstring(strptr)?.to_str()?.to_owned())
}

/// Convenience function to convert directly from a Rust `&str` to a
/// `std::ffi::CString`, i.e. a lifetime safe object capable of providing a raw
/// C string (`*const c_char` in Rust, `const char *` in C).
pub fn strref_to_cstring(strref: &str) -> Result<CString> {
  Ok(CString::new(strref)?)
}

/// Converts a Rust `std::path::Path` to a `std::ffi::CString` object. This is
/// helpful here since the original C library takes absolute paths (this
/// function takes any path) as `*const c_char` (`const char *` in C).
pub fn path_to_cstring(path: &Path) -> Result<CString> {
  strref_to_cstring(path.canonicalize()?
                        .to_str()
                        .ok_or(anyhow!("path '{}' invalid", path.display()))?)
}


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;


  #[test]
  fn strptr_to_test() {
    let as_strref = "warblgarbl";
    let as_cstring = CString::new(as_strref).unwrap();

    let conv_to_cstring = strptr_to_cstring(as_cstring.as_ptr()).unwrap();
    assert_eq!(as_strref, conv_to_cstring.to_str().unwrap());

    let conv_to_string = strptr_to_string(as_cstring.as_ptr()).unwrap();
    assert_eq!(as_strref, conv_to_string.as_str());
  }

  #[test]
  fn path_to_cstring_test() {
    let path_str = "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";
    let path_abs = Path::new(path_str).canonicalize().unwrap();

    let as_cstring = CString::new(path_abs.to_str().unwrap()).unwrap();
    assert_eq!(as_cstring, path_to_cstring(Path::new(path_str)).unwrap());
  }
}
