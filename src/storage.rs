// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use super::xdrkfile::XdrkFile;
use fubar::Result;
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


/// Holds metadata of a channel and does lazy loading of the channel data
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct Channel<'a> {
  #[getset(get = "pub")]
  name: String,
  #[getset(get_copy = "pub")]
  idx:  usize,
  #[getset(get = "pub")]
  unit: String,
  #[getset(get = "pub")]
  file: &'a XdrkFile,
  data: Option<ChannelData>,
}

impl<'a> Channel<'a> {
  pub fn new(name: String,
             idx: usize,
             unit: String,
             file: &'a XdrkFile)
             -> Self
  {
    Self { name,
           idx,
           unit,
           file,
           data: None }
  }

  pub fn data(&mut self) -> Result<&ChannelData> {
    if self.data.is_none() {
      self.data = Some(self.file.channel_samples(self.idx)?);
    }
    Ok(self.data.as_ref().unwrap())
  }
}
