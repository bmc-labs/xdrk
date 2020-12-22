// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use getset::{CopyGetters, Getters, MutGetters};
use std::{iter, vec};


const FREQUENCIES: [usize; 10] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000];


/// Holds raw, unsynchronized data of a channel and additional metadata.
#[derive(Clone, Debug, PartialEq, CopyGetters, Getters)]
#[getset(get = "pub")]
pub struct RawChannel {
  name: String,
  unit: String, // TODO eventually should be kissunits (crate) (?)
  data: RawChannelData,
}

impl RawChannel {
  pub fn new(name: String, unit: String, data: RawChannelData) -> Self {
    Self { name, unit, data }
  }

  /// Calculates and returns the recording frequency of the data in Hz.
  pub fn frequency(&self) -> f64 {
    if self.is_empty()
       || self.len() < 3
       || self.data.timestamps().iter().sum::<f64>() < 0.1
    {
      return 0.0;
    }

    // we multiply by 500 and divide back through 1000 for normalization on
    // milliseconds in three time steps. remember this is integer division so
    // this doesn't cancel.
    let (first, second) = (self.data.timestamps[0], self.data.timestamps[2]);
    let raw_frequency = (1_000.0 / ((second - first) * 500.0)).round() as i32;

    FREQUENCIES.iter()
               .min_by_key(|&&frequency| {
                 (raw_frequency - frequency as i32).abs()
               })
               .unwrap_or(&0_usize)
               .clone() as f64
  }

  pub fn len(&self) -> usize {
    self.data().len()
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}


/// Holds data of a channel retrieved from a file.
#[derive(Clone, Debug, PartialEq, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct RawChannelData {
  timestamps: Vec<f64>,
  samples:    Vec<f64>,
}

impl RawChannelData {
  /// Helper function which allocates memory buffers in the required format.
  pub fn allocate(count: usize) -> (Vec<f64>, Vec<f64>) {
    (vec![0.0; count], vec![0.0; count])
  }

  /// Creates a new `RawChannelData` object from buffers `t` (timestamps), `s`
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

impl IntoIterator for RawChannelData {
  type IntoIter = iter::Zip<vec::IntoIter<f64>, vec::IntoIter<f64>>;
  type Item = (f64, f64);

  fn into_iter(self) -> Self::IntoIter {
    self.timestamps.into_iter().zip(self.samples.into_iter())
  }
}


#[cfg(test)]
mod tests {
  use super::{super::XdrkFile, *};
  use pretty_assertions::{assert_eq, assert_ne};
  use std::path::Path;


  const XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn raw_channel_test() {
    // happy path tests without context
    let size = 42;
    let raw_channel_data = RawChannelData::from_tsc(Vec::with_capacity(size),
                                                    Vec::with_capacity(size),
                                                    size);
    let raw_channel = RawChannel::new("warbl".to_string(),
                                      "garbl".to_string(),
                                      raw_channel_data.clone());
    assert_eq!("warbl", raw_channel.name());
    assert_eq!("garbl", raw_channel.unit());
    assert_eq!(0.0, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(size, raw_channel.len());

    let new_channel = RawChannel::new("foo".to_string(),
                                      "bar".to_string(),
                                      raw_channel_data.clone());
    assert_ne!(raw_channel, new_channel);

    let raw_channel = new_channel;
    assert_eq!("foo", raw_channel.name());
    assert_eq!("bar", raw_channel.unit());
    assert_eq!(0.0, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(size, raw_channel.len());

    // tests with context
    let raw_channel = XdrkFile::load(Path::new(XRK_PATH)).unwrap()
                                                         .raw_channel(2, None)
                                                         .unwrap();
    assert_eq!("pManifoldScrut", raw_channel.name());
    assert_eq!("bar", raw_channel.unit());
    assert_eq!(100.0, raw_channel.frequency());
    assert_eq!(false, raw_channel.is_empty());
    assert_eq!(57980, raw_channel.len());
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_first_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic = RawChannelData::from_tsc(timestamps.clone(),
                                          samples.clone(),
                                          first_size);
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_second_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic = RawChannelData::from_tsc(timestamps.clone(),
                                          samples.clone(),
                                          second_size);
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_third_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic =
      RawChannelData::from_tsc(timestamps.clone(), samples.clone(), 1234);
  }

  #[test]
  #[should_panic]
  fn channel_data_len_panic_test() {
    let (timestamps, samples) = RawChannelData::allocate(42);
    let mut channel_data = RawChannelData::from_tsc(timestamps, samples, 42);

    // add timestamp and assert that we know panic when asking for len because
    // we have more timestamps than samples
    channel_data.timestamps_mut().push(123.456);
    let _panic = channel_data.len();
  }

  #[test]
  fn channel_data_test() {
    // test without context
    let (timestamps, samples) = RawChannelData::allocate(42);
    assert_eq!(42, timestamps.capacity());
    assert_eq!(42, samples.capacity());

    let channel_data = RawChannelData::from_tsc(timestamps, samples, 42);
    assert_eq!(42, channel_data.timestamps().len());
    assert_eq!(42, channel_data.samples().len());

    let (timestamps, samples) = RawChannelData::allocate(1337);
    let other_data = RawChannelData::from_tsc(timestamps, samples, 1337);
    assert_ne!(channel_data, other_data);

    // tests with context from test data
    let channel_data = XdrkFile::load(Path::new(XRK_PATH)).unwrap()
                                                          .raw_channel(2, None)
                                                          .unwrap()
                                                          .data()
                                                          .clone();
    assert_eq!(false, channel_data.is_empty());
    assert_eq!(57980, channel_data.timestamps().len());
    assert_eq!(57980, channel_data.samples().len());
  }
}
