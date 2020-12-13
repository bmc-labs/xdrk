// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use getset::{Getters, MutGetters};


/// Holds data of a channel and additional metadata
#[derive(Debug, PartialEq, Getters)]
pub struct Channel {
  #[getset(get = "pub")]
  name: String,
  #[getset(get = "pub")]
  data: ChannelData,
}

impl Channel {
  pub fn new(name: String, data: ChannelData) -> Self {
    Self { name, data }
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
  pub fn allocate(count: usize) -> (Vec<f64>, Vec<f64>) {
    (Vec::with_capacity(count), Vec::with_capacity(count))
  }

  /// Creates a new `ChannelData` object from buffers and a given buffer size
  pub fn from_tsc(mut timestamps: Vec<f64>,
                  mut samples: Vec<f64>,
                  count: usize)
                  -> Self
  {
    unsafe {
      timestamps.set_len(count);
      samples.set_len(count);
    }

    Self { timestamps,
           samples }
  }

  pub fn is_empty(&self) -> bool {
    self.timestamps.is_empty() && self.samples.is_empty()
  }
}
