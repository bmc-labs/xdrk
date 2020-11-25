// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};

#[derive(Debug, PartialEq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LapInfo {
  start:    f64,
  duration: f64,
}

impl LapInfo {
  pub fn new(start: f64, duration: f64) -> Self {
    Self { start, duration }
  }
}


#[derive(Debug, PartialEq, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct ChannelData {
  timestamps: Vec<f64>,
  samples:    Vec<f64>,
}

impl ChannelData {
  pub fn from_ts(timestamps: Vec<f64>, samples: Vec<f64>) -> Self {
    Self { timestamps,
           samples }
  }

  pub fn is_empty(&self) -> bool {
    self.timestamps.is_empty() && self.samples.is_empty()
  }
}
