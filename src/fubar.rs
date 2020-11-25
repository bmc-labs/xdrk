// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Jannik Schütz <jannik@bmc-labs.com>

use serde::Deserialize;
use std::{error, ffi, fmt, result, str};


/// libanna's result type `Result` will work with any error type
/// implementing the `std::error::Error` trait.
pub type Result<T> = result::Result<T, Fubar>;


#[derive(Clone, Debug, Deserialize, PartialEq)]
/// Simple error to be used throughout libanna to bubble errors back to the
/// respective main functions.
///
/// It is strongly recommended to use `Fubar` through the `fubar!` macro,
/// which makes life just so much easier as the `fubar!` macro accepts the
/// same parameters as the `format!` macro but returns an `Err(Fubar)`. See the
/// macro documentation for a code example.
///
/// FUBAR: Fucked Up Beyond All {Recognition, Repair, Reason}
pub struct Fubar(pub String);

impl Fubar {
  pub fn new(msg: &str) -> Self {
    Self(msg.to_string())
  }
}

/// The following traits - `fmt::Display` and `error::Error` - are required in
/// addition to deriving the `Debug` trait for `Fubar` to implement the
/// `error::Error` trait fully.
impl fmt::Display for Fubar {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl error::Error for Fubar {}


/// This macro - internal use only - generates the implementation of the
/// `From` trait for `Fubar` for a given list of types.
macro_rules! implement_from {
  ($($ErrType:ty),*) => {$(
    impl From<$ErrType> for Fubar {
      fn from(error: $ErrType) -> Self {
        Self(error.to_string())
      }
    }
  )*}
}

// here the macro is called with a list of types used in our codebase
implement_from!(chrono::ParseError,
                std::io::Error,
                str::Utf8Error,
                ffi::NulError);


/// The `fubar!` macro provides an easy way to return formatted errors
/// from functions returning a `Result`. It takes something which can be
/// formatted using the `format!` macro and returns an `Err(Fubar)`. You can
/// use it in your code as follows:
///
/// ```ignore
/// match something {
///   Ok(()) => Ok(()),  // the world is a happy place
///   Err(err) => fubar!("error \"{}\" could not be resolved", err),
/// }
/// ```
#[macro_export]
macro_rules! fubar {
  ($($arg:tt)*) => {
      Err($crate::fubar::Fubar(format!($($arg)*)))
  }
}


/// The `ensure!` macro provides and easy way to make sure a condition is true,
/// and if not, return an `Err(Fubar)` (exactly as `fubar!` does - `ensure!` is
/// actually implemented on top of `foobar!`). Use it as follows:
///
/// ```ignore
/// fn my_function(&self) -> Result<()> {
///   ensure!(self.has_enough_fish(), "sorry, only {} fish", self.fish());
/// }
/// ```
#[macro_export]
macro_rules! ensure {
  ($cond:expr, $($arg:tt)*) => {
    if !($cond) { return fubar!($($arg)*) }
  }
}


#[cfg(test)]
mod test {
  use super::{Fubar, Result};

  #[test]
  fn fubar_test() {
    let test_str = "warblgarbl";
    let err = Fubar(test_str.to_string());

    assert_eq!(test_str, &format!("{}", err));
    assert_eq!(fubar!("{}", test_str) as Result<()>, Err(err.clone()));
    assert_eq!(fubar!("warblgarbl") as Result<()>, Err(err.clone()));
    assert_eq!(fubar!("") as Result<()>, Err(Fubar("".to_string())));
  }

  #[test]
  fn ensure_test() {
    fn wrapper(cond: bool, msg: &str) -> Result<()> {
      assert_eq!(ensure!(cond, "{}", msg), ());
      Ok(())
    }

    let test_str = "warblgarbl";
    let err = Fubar(test_str.to_string());

    assert_eq!(wrapper(true, test_str), Ok(()));
    assert_eq!(wrapper(false, test_str), Err(err));
  }
}
