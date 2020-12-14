// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};


/// Holds data of a channel and additional metadata.
#[derive(Clone, Debug, PartialEq, Getters)]
#[getset(get = "pub")]
pub struct Channel {
  info: ChannelInfo,
  data: ChannelData,
}

impl Channel {
  pub fn new(info: ChannelInfo, data: ChannelData) -> Self {
    assert_eq!(info.samples_count(), data.len());
    Self { info, data }
  }

  /// Construct a new `Channel` from raw `ChannelInfo` input parameters:
  /// - name,
  /// - unit,
  /// - samples count (short: _count_)
  /// and, of course, a `ChannelData object`.
  pub fn from_nucd(name: String,
                   unit: String,
                   count: usize,
                   data: ChannelData)
                   -> Self
  {
    assert_eq!(count, data.len());
    Self { info: ChannelInfo::new(name, unit, count),
           data }
  }

  pub fn name(&self) -> &str {
    self.info.name()
  }

  pub fn unit(&self) -> &str {
    self.info.unit()
  }

  /// Calculates and returns the recording frequency of the data in Hz.
  pub fn frequency(&self) -> usize {
    if self.is_empty()
       || self.len() < 2
       || self.data.timestamps().iter().sum::<f64>() < 0.1
    {
      return 0;
    }

    let interval = self.data.timestamps()[1] - self.data.timestamps()[0];
    (1000.0 / (interval * 1000.0)).round() as usize
  }

  pub fn len(&self) -> usize {
    assert_eq!(self.info.samples_count(), self.data.timestamps().len());
    self.info.samples_count()
  }

  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}


/// Holds channel info
#[derive(Clone, Debug, PartialEq, CopyGetters, Getters)]
pub struct ChannelInfo {
  #[getset(get = "pub")]
  name:          String,
  #[getset(get = "pub")]
  unit:          String, // TODO eventually should be kissunits (crate) (?)
  #[getset(get_copy = "pub")]
  samples_count: usize,
}

impl ChannelInfo {
  pub fn new(name: String, unit: String, samples_count: usize) -> Self {
    Self { name,
           unit,
           samples_count }
  }
}


/// Holds data of a channel retrieved from a file.
#[derive(Clone, Debug, PartialEq, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct ChannelData {
  timestamps: Vec<f64>,
  samples:    Vec<f64>,
}

impl ChannelData {
  /// Helper function which allocates memory buffers in the required format.
  pub fn allocate(count: usize) -> (Vec<f64>, Vec<f64>) {
    (Vec::with_capacity(count), Vec::with_capacity(count))
  }

  /// Creates a new `ChannelData` object from buffers `t` (timestamps), `s`
  /// (samples) and a given buffer size `c` (capacity).
  pub fn from_tsc(mut timestamps: Vec<f64>,
                  mut samples: Vec<f64>,
                  capacity: usize)
                  -> Self
  {
    assert_eq!(timestamps.capacity(), capacity);
    assert_eq!(samples.capacity(), capacity);

    unsafe {
      timestamps.set_len(capacity);
      samples.set_len(capacity);
    }

    Self { timestamps,
           samples }
  }

  pub fn len(&self) -> usize {
    assert!(self.timestamps.len() == self.samples.len(),
            "number of timestamps not equivalent to number of samples");
    self.timestamps.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0usize
  }
}


#[cfg(test)]
mod tests {
  use super::{super::XdrkFile, *};
  use cool_asserts::assert_panics;
  use pretty_assertions::assert_eq;
  use std::path::Path;


  static XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn channel_test() {
    let (correct_size, panic_size) = (42, 1337);
    // unhappy path tests for constructors
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), panic_size);
    let channel_data = ChannelData::from_tsc(Vec::with_capacity(correct_size),
                                             Vec::with_capacity(correct_size),
                                             correct_size);
    assert_panics!(Channel::new(channel_info, channel_data.clone()));
    assert_panics!(Channel::from_nucd("warbl".to_string(),
                                      "garbl".to_string(),
                                      panic_size,
                                      channel_data.clone()));

    // happy path tests without context
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), correct_size);
    let channel = Channel::new(channel_info, channel_data.clone());
    assert_eq!("warbl", channel.name());
    assert_eq!("garbl", channel.unit());
    assert_eq!(0, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(correct_size, channel.len());

    let channel = Channel::from_nucd("warbl".to_string(),
                                     "garbl".to_string(),
                                     correct_size,
                                     channel_data.clone());
    assert_eq!("warbl", channel.name());
    assert_eq!("garbl", channel.unit());
    assert_eq!(0, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(correct_size, channel.len());

    // tests with context
    let channel = XdrkFile::load(Path::new(XRK_PATH)).unwrap()
                                                     .channel("pManifoldScrut")
                                                     .unwrap();
    assert_eq!("pManifoldScrut", channel.name());
    assert_eq!("bar", channel.unit());
    assert_eq!(100, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(57980, channel.len());
  }

  #[test]
  fn channel_info_test() {
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), 42);
    assert_eq!("warbl", channel_info.name());
    assert_eq!("garbl", channel_info.unit());
    assert_eq!(42, channel_info.samples_count());
  }

  #[test]
  fn channel_data_test() {
    // unhappy path test for constructor
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    assert_panics!(ChannelData::from_tsc(timestamps.clone(),
                                         samples.clone(),
                                         first_size));
    assert_panics!(ChannelData::from_tsc(timestamps.clone(),
                                         samples.clone(),
                                         second_size));
    assert_panics!(ChannelData::from_tsc(timestamps.clone(),
                                         samples.clone(),
                                         1234));

    // test without context
    let (timestamps, samples) = ChannelData::allocate(42);
    assert_eq!(42, timestamps.capacity());
    assert_eq!(42, samples.capacity());

    let mut channel_data = ChannelData::from_tsc(timestamps, samples, 42);
    assert_eq!(42, channel_data.timestamps().len());
    assert_eq!(42, channel_data.samples().len());

    // add timestamp and assert that we know panic when asking for len because
    // we have more timestamps than samples
    channel_data.timestamps_mut().push(123.456);
    assert_panics!(channel_data.len());
    assert_panics!(channel_data.is_empty());

    // tests with context from test data
    let channel_data =
      XdrkFile::load(Path::new(XRK_PATH)).unwrap()
                                         .channel("pManifoldScrut")
                                         .unwrap()
                                         .data()
                                         .clone();
    assert_eq!(false, channel_data.is_empty());
    assert_eq!(57980, channel_data.timestamps().len());
    assert_eq!(57980, channel_data.samples().len());
  }
}
