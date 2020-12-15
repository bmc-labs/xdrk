// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};
use std::{iter, vec};


const FREQUENCIES: [usize; 10] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000];


/// Holds synchronized channel data.
#[derive(Clone, Debug, PartialEq, CopyGetters, Getters)]
pub struct Channel {
  #[getset(get = "pub")]
  name:      String,
  #[getset(get = "pub")]
  unit:      String,
  #[getset(get_copy = "pub")]
  frequency: usize,
  #[getset(get = "pub")]
  data:      Vec<f32>,
}

impl Channel {
  pub fn from_raw_channel(raw: RawChannel, time: f64) -> Self {
    let name = raw.name().to_owned();
    let unit = raw.unit().to_owned();
    let frequency = raw.frequency();
    let mut data = Vec::with_capacity((time * frequency as f64) as usize);

    let advance = 1.0 / frequency as f64;
    let threshold = 0.5 * advance;

    let (mut timestamp, mut sample) = (0.0f64, 0.0f64);
    let mut raw_iter = raw.data().to_owned().into_iter();
    let mut raw_data = raw_iter.next();

    while timestamp < time {
      if raw_data.is_some()
         && (raw_data.unwrap().0 - timestamp).abs() < threshold
      {
        sample = raw_data.unwrap().1;
        raw_data = raw_iter.next();
      }
      timestamp = timestamp + advance;
      data.push(sample as f32);
    }

    Self { name,
           unit,
           frequency,
           data }
  }

  pub fn len(&self) -> usize {
    self.data.len()
  }

  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}


/// Holds raw, unsynchronized data of a channel and additional metadata.
#[derive(Clone, Debug, PartialEq, Getters)]
#[getset(get = "pub")]
pub struct RawChannel {
  name:  String,
  unit:  String, // TODO eventually should be kissunits (crate) (?)
  #[getset(get)] // eliminate getter for count: use `len()` instead
  count: usize,
  data:  ChannelData,
}

impl RawChannel {
  pub fn new(info: ChannelInfo, data: ChannelData) -> Self {
    assert_eq!(info.count, data.len());
    Self { name: info.name,
           unit: info.unit,
           count: info.count,
           data }
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
    Self { name,
           unit,
           count,
           data }
  }

  /// Calculates and returns the recording frequency of the data in Hz.
  pub fn frequency(&self) -> usize {
    if self.is_empty()
       || self.len() < 2
       || self.data.timestamps().iter().sum::<f64>() < 0.1
    {
      return 0;
    }

    // we multiply by 1000 and divide back through it for normalization on
    // milliseconds. remember this is integer division so this doesn't cancel.
    let (first, second) = (self.data.timestamps[0], self.data.timestamps[1]);
    let raw_frequency = (1000.0 / ((second - first) * 1000.0)).round();

    FREQUENCIES.iter()
               .find(|&&frequency| {
                 (raw_frequency - frequency as f64).abs()
                 < (0.25 * frequency as f64) // ±25%
               })
               .unwrap_or(&0usize)
               .clone()
  }

  pub fn len(&self) -> usize {
    assert_eq!(self.count, self.data.timestamps().len());
    self.count
  }

  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}


/// Holds channel info
#[derive(Clone, Debug, PartialEq, CopyGetters, Getters)]
pub struct ChannelInfo {
  #[getset(get = "pub")]
  name:  String,
  #[getset(get = "pub")]
  unit:  String, // TODO eventually should be kissunits (crate) (?)
  #[getset(get_copy = "pub")]
  count: usize,
}

impl ChannelInfo {
  pub fn new(name: String, unit: String, count: usize) -> Self {
    Self { name, unit, count }
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

impl IntoIterator for ChannelData {
  type IntoIter = iter::Zip<vec::IntoIter<f64>, vec::IntoIter<f64>>;
  type Item = (f64, f64);

  fn into_iter(self) -> Self::IntoIter {
    self.timestamps.into_iter().zip(self.samples.into_iter())
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
    let xdrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();

    let raw_channel = xdrk_file.raw_channel("pManifoldScrut").unwrap();
    let channel = Channel::from_raw_channel(raw_channel, 580.205);
    assert_eq!("pManifoldScrut", channel.name());
    assert_eq!("bar", channel.unit());
    assert_eq!(100, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(58021, channel.len());

    let raw_channel = xdrk_file.raw_channel_in_lap("fEngRpm", 1).unwrap();
    let channel = Channel::from_raw_channel(raw_channel, 133.749);
    assert_eq!("fEngRpm", channel.name());
    assert_eq!("rpm", channel.unit());
    assert_eq!(100, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(13375, channel.len());
  }

  #[test]
  fn raw_channel_test() {
    let (correct_size, panic_size) = (42, 1337);
    // unhappy path tests for constructors
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), panic_size);
    let raw_channel_data =
      ChannelData::from_tsc(Vec::with_capacity(correct_size),
                            Vec::with_capacity(correct_size),
                            correct_size);
    assert_panics!(RawChannel::new(channel_info, raw_channel_data.clone()));
    assert_panics!(RawChannel::from_nucd("warbl".to_string(),
                                         "garbl".to_string(),
                                         panic_size,
                                         raw_channel_data.clone()));

    // happy path tests without context
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), correct_size);
    let raw_channel = RawChannel::new(channel_info, raw_channel_data.clone());
    assert_eq!("warbl", raw_channel.name());
    assert_eq!("garbl", raw_channel.unit());
    assert_eq!(0, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(correct_size, raw_channel.len());

    let raw_channel = RawChannel::from_nucd("warbl".to_string(),
                                            "garbl".to_string(),
                                            correct_size,
                                            raw_channel_data.clone());
    assert_eq!("warbl", raw_channel.name());
    assert_eq!("garbl", raw_channel.unit());
    assert_eq!(0, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(correct_size, raw_channel.len());

    // tests with context
    let raw_channel =
      XdrkFile::load(Path::new(XRK_PATH)).unwrap()
                                         .raw_channel("pManifoldScrut")
                                         .unwrap();
    assert_eq!("pManifoldScrut", raw_channel.name());
    assert_eq!("bar", raw_channel.unit());
    assert_eq!(100, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(57980, raw_channel.len());
  }

  #[test]
  fn channel_info_test() {
    let channel_info =
      ChannelInfo::new("warbl".to_string(), "garbl".to_string(), 42);
    assert_eq!("warbl", channel_info.name);
    assert_eq!("garbl", channel_info.unit);
    assert_eq!(42, channel_info.count);
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
                                         .raw_channel("pManifoldScrut")
                                         .unwrap()
                                         .data()
                                         .clone();
    assert_eq!(false, channel_data.is_empty());
    assert_eq!(57980, channel_data.timestamps().len());
    assert_eq!(57980, channel_data.samples().len());
  }
}
