// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

mod xdrk_bindings;
pub use xdrk_bindings::*;

use std::ffi::CStr;


#[derive(Debug)]
pub struct XdrkFile {
  idx: usize,
}


impl XdrkFile {
  // pub fn new()

  /// Returns the compile date of this library
  pub fn library_date() -> String {
    unsafe { CStr::from_ptr(get_library_date()) }.to_str()
                                                 .expect("error parsing date")
                                                 .to_string()
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn meta_functions() {
    assert_eq!("Jan 24 2020".to_string(), XdrkFile::library_date());
  }
}
