// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::Channel;
use getset::{CopyGetters, Getters};


/// Hold all channels of a lap.
#[derive(Debug, PartialEq, Getters)]
pub struct Lap {
  info:     LapInfo,
  channels: Vec<Channel>,
}

impl Lap {
  pub fn new(info: LapInfo, channels: Vec<Channel>) -> Self {
    Self { info, channels }
  }
}

/// Stores the start time within the recording and the time of a lap.
#[derive(Debug, PartialEq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LapInfo {
  number: usize,
  start:  f64,
  time:   f64,
}

impl LapInfo {
  pub fn new(number: usize, start: f64, time: f64) -> Self {
    Self { number,
           start,
           time }
  }
}
