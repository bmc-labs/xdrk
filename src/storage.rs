// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};

/// Stores the start time within the recording and the duration of a lap.
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


/// Holds data of a channel retrieved from a file.
#[derive(Debug, PartialEq, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct ChannelData {
  timestamps: Vec<f64>,
  samples:    Vec<f64>,
}

impl ChannelData {
  /// Helper function which allocates memory buffers in the required format
  pub fn allocate(capacity: usize) -> (Vec<f64>, Vec<f64>) {
    (Vec::with_capacity(capacity), Vec::with_capacity(capacity))
  }

  /// Creates a new `ChannelData` object from buffers and a given buffer size
  pub fn from_tsc(mut timestamps: Vec<f64>,
                  mut samples: Vec<f64>,
                  size: usize)
                  -> Self
  {
    unsafe {
      timestamps.set_len(size);
      samples.set_len(size);
    }

    Self { timestamps,
           samples }
  }

  pub fn is_empty(&self) -> bool {
    self.timestamps.is_empty() && self.samples.is_empty()
  }
}
