// Copyright 2021 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <alumni@bmc-labs.com>

use eyre::{ensure, Result};
use getset::{CopyGetters, Getters, MutGetters};
use std::{iter, vec};


const FREQUENCIES: [usize; 10] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000];


/// Holds raw, unsynchronized data of a channel and additional metadata.
#[derive(Clone, Debug, Default, PartialEq, CopyGetters, Getters)]
#[getset(get = "pub")]
pub struct Channel {
  name: String,
  unit: String, // TODO eventually should be kissunits (crate) (?)
  data: ChannelData,
}

impl Channel {
  pub fn new(name: String, unit: String, data: ChannelData) -> Self {
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
    // it doesn't cancel.
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

  /// Synchronize the channel with another channel, i.e. form a channel with
  /// the same frequency and timestamps and corresponding values.
  ///
  /// ## Function
  ///
  /// This uses the timestamps of channel `other`. The newly created channel
  ///
  /// - has linearly interpolated samples if `other` has a higher frequency or
  ///   if the distance between a timestamp and the corresponding timestamp in
  ///   `other` is bigger than `0.5 * (1 / frequency)`
  /// - skips all samples for timestamps with no corresponding timestamp or
  ///   pair of timestamps in `other`
  ///
  /// ## Fails if
  ///
  /// - this channel or `other` contain less than 3 data points
  /// - this channel and `other` have non-intersecting timestamp ranges
  pub fn synchronize_with(&self, other: &Self) -> Result<Self> {
    const DNCSDP: &str = "does not contain sufficient data points";
    ensure!(self.len() >= 3, format!("channel {}", DNCSDP));
    ensure!(other.len() >= 3, format!("other {}", DNCSDP));

    // these unwraps are safe: we just tested that the channels are not empty,
    // which would be the only way for the `.first()` and `.last()` calls to
    // return a `None` value and thus cause a panic on unwrap
    ensure!(self.data.timestamps().first().unwrap()
            <= other.data.timestamps().last().unwrap()
            && self.data.timestamps().last().unwrap()
               >= other.data.timestamps().first().unwrap(),
            "channels have non-intersecting timestamp ranges");

    let (mut idx, timestamps, samples) =
      (0usize, self.data.timestamps(), self.data.samples());

    let mut new_sample: f64;
    let mut new_samples = Vec::with_capacity(other.len());
    let threshold = 0.5 / other.frequency();

    // we create a new sample for each timestamp in other
    for ref_ts in other.data.timestamps() {
      // first we look for a timestamp which is later then the current ref_ts
      new_sample = match timestamps[idx..].iter()
                                          .position(|v| v > ref_ts)
                                          .and_then(|pos| Some(idx + pos))
      {
        // if we find one and it's the first one and we are still at the start:
        Some(pos) if pos == 0 => samples[0],
        // if we find one and we're not at the start:
        Some(pos) => {
          // step back one sample, just before the current ref_ts...
          idx = pos - 1;
          if (ref_ts - timestamps[idx]).abs() <= threshold {
            // and if that is closer than threshold, we use the value...
            samples[idx]
          } else if (ref_ts - timestamps[idx + 1]).abs() <= threshold {
            // if the sample after the current ref_ts is closer, use that...
            samples[idx + 1]
          } else {
            // otherwise, interpolate between the samples of the data point
            // before and after the current ref_ts.
            samples[idx]
            + ((samples[idx + 1] - samples[idx])
               / (timestamps[idx + 1] - timestamps[idx]))
              * (ref_ts - timestamps[idx])
          }
        }
        // if we don't find one, i.e. we're now "backfilling":
        None => samples[samples.len() - 1],
      };

      // push the new sample
      new_samples.push(new_sample);
    }

    Ok(Self::new(self.name.clone(),
                 self.unit.clone(),
                 ChannelData::from_tsc(other.data().timestamps().clone(),
                                       new_samples,
                                       other.len())))
  }
}


/// Holds data of a channel retrieved from a file.
#[derive(Clone, Debug, Default, PartialEq, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct ChannelData {
  timestamps: Vec<f64>,
  samples:    Vec<f64>,
}

impl ChannelData {
  /// Helper function which allocates memory buffers in the required format.
  pub fn allocate(count: usize) -> (Vec<f64>, Vec<f64>) {
    (vec![0.0; count], vec![0.0; count])
  }

  /// Creates a new `ChannelData` object from buffers `t` (timestamps), `s`
  /// (samples) and a given buffer size `c` (capacity).
  pub fn from_tsc(mut timestamps: Vec<f64>,
                  mut samples: Vec<f64>,
                  capacity: usize)
                  -> Self {
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
  use super::{super::Run, *};
  use pretty_assertions::{assert_eq, assert_ne};
  use std::path::Path;


  const XRK_PATH: &str =
    "./testdata/032/TCR_EU-21_E02-LCA_Q1_AU-RS3-R5-S-S_032_A_1375.xrk";

  #[test]
  fn channel_test() {
    // happy path tests without context
    let size = 42;
    let channel_data =
      ChannelData::from_tsc(vec![0.0; size], vec![0.0; size], size);
    let channel = Channel::new("warbl".to_string(),
                               "garbl".to_string(),
                               channel_data.clone());
    assert_eq!("warbl", channel.name());
    assert_eq!("garbl", channel.unit());
    assert_eq!(0.0, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(size, channel.len());

    let new_channel =
      Channel::new("foo".to_string(), "bar".to_string(), channel_data.clone());
    assert_ne!(channel, new_channel);

    let channel = new_channel;
    assert_eq!("foo", channel.name());
    assert_eq!("bar", channel.unit());
    assert_eq!(0.0, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(size, channel.len());

    // tests with context
    let run = Run::load(Path::new(XRK_PATH)).unwrap();
    let channel = run.channel(2, None).unwrap();

    assert_eq!("pManifoldScrut", channel.name());
    assert_eq!("bar", channel.unit());
    assert_eq!(100.0, channel.frequency());
    assert_eq!(false, channel.is_empty());
    assert_eq!(70588, channel.len());
  }

  #[test]
  fn sync_test() {
    let run = Run::load(Path::new(XRK_PATH)).unwrap();
    let p_brake = run.channel(run.channel_idx("pBrakeF").unwrap(), Some(2))
                     .unwrap();
    let v_wheel = run.channel(run.channel_idx("vWheelFL").unwrap(), Some(2))
                     .unwrap();

    let p_brake_sync = p_brake.synchronize_with(&v_wheel).unwrap();
    assert_eq!(v_wheel.len(), p_brake_sync.len());
    assert_eq!(v_wheel.data().timestamps(),
               p_brake_sync.data().timestamps());

    // let p_brake_data = p_brake.data()
    // .timestamps()
    // .iter()
    // .zip(p_brake.data().samples())
    // .map(|(ts, s)| format!("{},{}", ts, s))
    // .collect::<Vec<_>>()
    // .join("\n");
    // std::fs::write("p_brake_data.csv", p_brake_data).unwrap();
    //
    // let p_brake_sync_data = p_brake_sync.data()
    // .timestamps()
    // .iter()
    // .zip(p_brake_sync.data().samples())
    // .map(|(ts, s)| format!("{},{}", ts, s))
    // .collect::<Vec<_>>()
    // .join("\n");
    // std::fs::write("p_brake_sync_data.csv", p_brake_sync_data).unwrap();
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_first_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic =
      ChannelData::from_tsc(timestamps.clone(), samples.clone(), first_size);
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_second_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic =
      ChannelData::from_tsc(timestamps.clone(), samples.clone(), second_size);
  }

  #[test]
  #[should_panic]
  fn channel_data_from_tsc_panic_third_test() {
    let (first_size, second_size) = (42, 1337);
    let (timestamps, samples) =
      (Vec::with_capacity(first_size), Vec::with_capacity(second_size));
    let _panic =
      ChannelData::from_tsc(timestamps.clone(), samples.clone(), 1234);
  }

  #[test]
  #[should_panic]
  fn channel_data_len_panic_test() {
    let (timestamps, samples) = ChannelData::allocate(42);
    let mut channel_data = ChannelData::from_tsc(timestamps, samples, 42);

    // add timestamp and assert that we know panic when asking for len because
    // we have more timestamps than samples
    channel_data.timestamps_mut().push(123.456);
    let _panic = channel_data.len();
  }

  #[test]
  fn channel_data_test() {
    // test without context
    let (timestamps, samples) = ChannelData::allocate(42);
    assert_eq!(42, timestamps.capacity());
    assert_eq!(42, samples.capacity());

    let channel_data = ChannelData::from_tsc(timestamps, samples, 42);
    assert_eq!(42, channel_data.timestamps().len());
    assert_eq!(42, channel_data.samples().len());

    let (timestamps, samples) = ChannelData::allocate(1337);
    let other_data = ChannelData::from_tsc(timestamps, samples, 1337);
    assert_ne!(channel_data, other_data);

    // tests with context from test data
    let channel_data = Run::load(Path::new(XRK_PATH)).unwrap()
                                                     .channel(2, None)
                                                     .unwrap()
                                                     .data()
                                                     .clone();
    assert_eq!(false, channel_data.is_empty());
    assert_eq!(70588, channel_data.timestamps().len());
    assert_eq!(70588, channel_data.samples().len());
  }
}
