// Copyright 2021 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <alumni@bmc-labs.com>

use super::Channel;
use getset::{CopyGetters, Getters};


/// Hold all channels of a lap.
#[derive(Debug, PartialEq, CopyGetters, Getters)]
pub struct Lap {
  #[getset(get_copy = "pub")]
  id:       usize,
  #[getset(get_copy = "pub")]
  start:    f64,
  #[getset(get_copy = "pub")]
  time:     f64,
  #[getset(get = "pub")]
  channels: Vec<Channel>,
}

impl Lap {
  pub fn new(info: LapInfo, channels: Vec<Channel>) -> Self {
    Self { id: info.id(),
           start: info.start(),
           time: info.time(),
           channels }
  }

  pub fn channel_names(&self) -> Vec<String> {
    self.channels
        .iter()
        .map(|channel| channel.name().clone())
        .collect()
  }

  pub fn channel(&self, name: &str) -> Option<&Channel> {
    self.channels.iter().find(|c| c.name() == name)
  }

  pub fn max_frequency(&self) -> f64 {
    if self.channels.is_empty() {
      return 0.0;
    }
    self.channels
        .iter()
        .max_by_key(|channel| channel.frequency() as usize)
        .unwrap()
        .frequency()
  }
}

/// Stores the start time within the recording and the time of a lap.
#[derive(Debug, Clone, Copy, PartialEq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LapInfo {
  id:    usize,
  start: f64,
  time:  f64,
}

impl LapInfo {
  pub fn new(id: usize, start: f64, time: f64) -> Self {
    Self { id, start, time }
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
  fn lap_test() {
    let run = Run::load(Path::new(XRK_PATH)).unwrap();

    let lap = run.lap(1).unwrap();
    assert_eq!(1, lap.id());
    assert_eq!(201.243, lap.start());
    assert_eq!(134.936, lap.time());

    macro_rules! stringvec {
      ($($x:literal),* $(,)?) => (vec![$($x.to_string()),*]);
    }
    #[cfg(target_family = "unix")]
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
                                   "swGearDown",
                                   "swGearUp",
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
                                   "momEngTorq",
                                   "momEngTorqTarget",
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
    #[cfg(target_family = "windows")]
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
                                   "swGearDown",
                                   "swGearUp",
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
                                   "momEngTorq",
                                   "momEngTorqTarget",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",
                                   "",];
    assert_eq!(channel_names, lap.channel_names());

    let p_manifold_scrut = lap.channel("pManifoldScrut").unwrap();
    assert_eq!("pManifoldScrut", p_manifold_scrut.name());
    assert_eq!(100.0, p_manifold_scrut.frequency());

    assert_eq!(100.0, lap.max_frequency());
    assert_ne!(lap, run.lap(2).unwrap());

    let lap = Lap::new(LapInfo::new(0, 0.0, 0.0), Vec::new());
    assert_eq!(0.0, lap.max_frequency());
  }

  #[test]
  fn lap_info_test() {
    let lap_info = LapInfo::new(2, 145.156, 133.135);
    assert_eq!(2, lap_info.id());
    assert_eq!(145.156, lap_info.start());
    assert_eq!(133.135, lap_info.time());
  }
}
