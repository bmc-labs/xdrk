// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};


/// Holds data of a channel and additional metadata
#[derive(Debug, PartialEq, Getters)]
#[getset(get = "pub")]
pub struct Channel {
  info: ChannelInfo,
  data: ChannelData,
}

impl Channel {
  pub fn new(info: ChannelInfo, data: ChannelData) -> Self {
    Self { info, data }
  }

  pub fn from_infos(name: String,
                    unit: String,
                    samples_count: usize,
                    data: ChannelData)
                    -> Self
  {
    Self { info: ChannelInfo::new(name, unit, samples_count),
           data }
  }

  pub fn frequency(&self) -> usize {
    if self.data.is_empty() {
      return 0usize;
    }
    42usize
  }

  pub fn len(&self) -> usize {
    self.data.timestamps().len()
  }
}


/// Holds channel info
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct ChannelInfo {
  #[getset(get = "pub")]
  name:         String,
  #[getset(get = "pub")]
  unit:         String, // TODO eventually should be kissunits (crate) (?)
  #[getset(get_copy = "pub")]
  sample_count: usize,
}

impl ChannelInfo {
  pub fn new(name: String, unit: String, sample_count: usize) -> Self {
    Self { name,
           unit,
           sample_count }
  }

  pub fn destruct(self) -> (String, String, usize) {
    (self.name, self.unit, self.sample_count)
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
