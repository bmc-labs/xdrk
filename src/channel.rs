// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::RawChannel;
use getset::{CopyGetters, Getters};


/// Holds synchronized channel data.
///
/// That means that there is some background magic going on to get a `Channel`
/// from a `RawChannel`. This is necessary because by default, XRK/DRK files
/// _do not contain the number of samples you would expect judging by the
/// polling rate / recording frequency you have set and the lap time_.
/// Specifically, recording starts late based on timestamp (i.e. when the
/// logger starts, a clock is started and whenever the measurement subsystem or
/// CAN subsystem or whatever is ready, that is when recordings start - not at
/// time 0, as you might expect), frequencies vary from what you set (i.e. if
/// you set a CAN channel to record at 10Hz but the CAN carries this channel at
/// 15Hz, it will still end up at 15Hz in the logs) and samples are not
/// recorded at all if they weren't seen for any reason (i.e. if there are
/// intermittent failures of the measurement subsystem of the logger or if CAN
/// messages are intermittently dropped, there will simply not be any entry in
/// the logging). All of this leads to channels recorded at the same frequency
/// (or rather, channels which are set up by the user to be recorded at the
/// same frequency) tend not to have the same number of samples in a lap.
///
/// Now, if you don't care about that in your use case, go ahead and use
/// `RawChannel`. If you do care: we convert `RawChannel` to `Channel` using
/// the following algorithm:
///
/// 1. Calculate the frequency based on the time gap between the first and the
///    third sample, and normalizing this to the frequencies available to the
///    user in AiM. The logic for this is in `RawChannel` since you may need it
///    there as well.
/// 2. Using the frequency and a given time, we
///     - fill the beginning of the log with as many zeroes as is appropriate,
///       i.e. until the first sample is close enough to the timestep
///       calculated from the frequency
///     - attach measurements to "clean" (i.e. calculated from frequency)
///       timegaps
///     - fill in gaps with the respective previous value (based on how
///       measurement devices normally operate, where the measurement subsystem
///       would write a measurement to memory via direct memory access at a
///       given frequency, and the recording subsystem would write the contents
///       of that memory address to disk at a given frequency, and whatever
///       overlaps - such as intermittent failures of the measurement subsystem
///       - occurred would be written off under _limitations of the measurement
///       process_)
/// 3. Discard all original timestamps, i.e. end up with a single column vector
///    of samples in which each samples corresponds to a timestamp based on its
///    index in the vector and the recording frequency.
///
/// This has turned out to perform well as AiM does well keeping the
/// measurement equidistant even though there are missing samples.
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
  pub fn from_raw_channel(raw: RawChannel, start: f64, time: f64) -> Self {
    let name = raw.name().to_owned();
    let unit = raw.unit().to_owned();
    let frequency = raw.frequency();
    let mut data =
      Vec::with_capacity((time * frequency as f64).ceil() as usize);

    let advance = 1.0 / frequency as f64;
    let threshold = 0.5 * advance;

    let (mut timestamp, mut sample) = (start, 0.0f64);
    let mut raw_iter = raw.data().to_owned().into_iter();
    let mut raw_data = raw_iter.next();

    while timestamp < (start + time) {
      if raw_data.is_some()
         && (raw_data.unwrap().0 - timestamp).abs() <= threshold
      {
        sample = raw_data.unwrap().1;
        raw_data = raw_iter.next();
      }
      timestamp += advance;
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


#[cfg(test)]
mod tests {
  use super::{super::XdrkFile, *};
  use pretty_assertions::{assert_eq, assert_ne};
  use std::path::Path;


  const XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn channel_test() {
    let xdrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();

    let raw_channel = xdrk_file.raw_channel(2, None).unwrap();
    let channel = Channel::from_raw_channel(raw_channel, 0.0, 580.205);
    assert_eq!("pManifoldScrut", channel.name());
    assert_eq!("bar", channel.unit());
    assert_eq!(100, channel.frequency());
    assert_eq!(58021, channel.len());
    assert_eq!(false, channel.is_empty());
    assert_eq!(0.0, channel.data()[0]);

    let raw_channel = xdrk_file.raw_channel(15, Some(1)).unwrap();
    let other_channel = Channel::from_raw_channel(raw_channel, 0.0, 133.749);
    assert_ne!(channel, other_channel);

    let channel = other_channel;
    assert_eq!("fEngRpm", channel.name());
    assert_eq!("rpm", channel.unit());
    assert_eq!(100, channel.frequency());
    assert_eq!(13375, channel.len());
    assert_eq!(false, channel.is_empty());
    assert_eq!(0.0, channel.data()[0]);
  }
}
