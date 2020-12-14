// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::{Channel, RawChannel};
use getset::{CopyGetters, Getters};


/// Hold all channels of a lap.
#[derive(Debug, PartialEq, Getters)]
pub struct Lap {
  number:   usize,
  start:    f64,
  time:     f64,
  channels: Vec<Channel>,
}

impl Lap {
  pub fn new(info: LapInfo, channels: Vec<Channel>) -> Self {
    Self { number: info.number(),
           start: info.start(),
           time: info.time(),
           channels }
  }

  pub fn from_raw(info: LapInfo, raw_channels: Vec<RawChannel>) -> Self {
    let channels =
      raw_channels.into_iter()
                  .map(|c| Channel::from_raw_channel(c, info.time()))
                  .collect();

    Self::new(info, channels)
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
