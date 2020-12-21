// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::{Channel, RawChannel};
use getset::{CopyGetters, Getters};


/// Hold all channels of a lap.
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct Lap {
  #[getset(get_copy = "pub")]
  number:   usize,
  #[getset(get_copy = "pub")]
  start:    f64,
  #[getset(get_copy = "pub")]
  time:     f64,
  #[getset(get = "pub")]
  channels: Vec<Channel>,
}

impl Lap {
  pub fn new(info: LapInfo, channels: Vec<Channel>) -> Self {
    Self { number: info.number(),
           start: info.start(),
           time: info.time(),
           channels }
  }

  pub fn from_raw_channels(info: LapInfo,
                           raw_channels: Vec<RawChannel>)
                           -> Self
  {
    let channels =
      raw_channels.into_iter()
                  .map(|c| Channel::from_raw_channel(c, info.time()))
                  .collect();
    Self::new(info, channels)
  }

  pub fn channel_names(&self) -> Vec<String> {
    self.channels
        .iter()
        .map(|channel| channel.name().clone())
        .collect()
  }

  pub fn frequency(&self) -> usize {
    if self.channels.is_empty() {
      return 0;
    }
    self.channels
        .iter()
        .max_by_key(|channel| channel.frequency())
        .unwrap()
        .frequency()
  }
}

/// Stores the start time within the recording and the time of a lap.
#[derive(Debug, PartialEq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LapInfo {
  number: usize,
  start:  f64,
  time:   f64,
}

impl LapInfo {
  pub fn new(number: usize, start: f64, time: f64) -> Self {
    Self { number,
           start,
           time }
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
  fn lap_test() {
    let xdrk_file = XdrkFile::load(Path::new(XRK_PATH)).unwrap();

    let lap = xdrk_file.lap(1).unwrap();
    assert_eq!(1, lap.number());
    assert_eq!(249.509, lap.start());
    assert_eq!(133.749, lap.time());

    macro_rules! stringvec {
      ($($x:literal),* $(,)?) => (vec![$($x.to_string()),*]);
    }
    let channel_names = stringvec!["Logger Temperature",
                                   "External Voltage",
                                   "pManifoldScrut",
                                   "tManifoldScrut",
                                   "aLon",
                                   "aLat",
                                   "aVer",
                                   "wRoll",
                                   "wPitch",
                                   "wYaw",
                                   "bAdvance",
                                   "bSteering",
                                   "bVvtIn",
                                   "bVvtOut",
                                   "dInjection",
                                   "fEngRpm",
                                   "pBrakeF",
                                   "pBrakeR",
                                   "pManifold",
                                   "posGear",
                                   "pRail",
                                   "rLambda",
                                   "rPedal",
                                   "rThrottle",
                                   "swLaunchState",
                                   "swRotFcy",
                                   "swRotPit",
                                   "tAmbient",
                                   "tManifold",
                                   "tWater",
                                   "uBarrel",
                                   "vWheelFL",
                                   "vWheelFR",
                                   "vWheelRL",
                                   "vWheelRR",
                                   "mEngTorq",
                                   "mEngTorqTarget",
                                   "posGearDSG",
                                   "swGearUP",
                                   "swGearDOWN",
                                   "GPS Speed",
                                   "GPS Nsat",
                                   "GPS LatAcc",
                                   "GPS LonAcc",
                                   "GPS Slope",
                                   "GPS Heading",
                                   "GPS Gyro",
                                   "GPS Altitude",
                                   "GPS PosAccuracy",
                                   "GPS SpdAccuracy",
                                   "GPS Radius",];
    assert_eq!(channel_names, lap.channel_names());

    for channel in lap.channels() {
      assert_eq!((lap.time() * channel.frequency() as f64).ceil() as usize,
                 channel.len());
    }
    assert_eq!(100, lap.frequency());
    assert_ne!(lap, xdrk_file.lap(2).unwrap());
  }

  #[test]
  fn lap_info_test() {
    let lap_info = LapInfo::new(2, 145.156, 133.135);
    assert_eq!(2, lap_info.number());
    assert_eq!(145.156, lap_info.start());
    assert_eq!(133.135, lap_info.time());
  }
}
