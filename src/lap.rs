// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::Channel;
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

  pub fn distance(&self) -> f64 {
    let v_gps = if let Some(v_gps) = self.channel("GPS Speed") {
      v_gps
    } else {
      return 0.0;
    };

    let stepsize = std::cmp::max(1, v_gps.frequency() as usize / 10);
    let (t, v) = (v_gps.data().timestamps(), v_gps.data().samples());

    let mut dist = 0.0;
    for i in (stepsize..v_gps.len()).step_by(stepsize) {
      // the following is the simple distance at constant acceleration formula:
      //
      //   x(t) = x_0 + v_0 * t + a_c * t^2
      //
      // this can be reformulated as a series:
      //
      //   x_i = x_(i - 1) + v_(i - 1) * Δt + 0.5 * a_i * (Δt)^2
      //   where i = 1, 2, ...
      //
      // with a_i = (v_i - v_(i - 1)) / Δt, we get
      //
      //   x_i = x_(i - 1) + v_(i - 1) * Δt + 0.5 * (v_i - v_(i - 1)) * Δt
      //   where i = 1, 2, ...
      //
      // and therefore
      //
      //   x_i = x_(i - 1) + 0.5 * (v_i + v_(i - 1)) * Δt
      //   where i = 1, 2, ...
      //
      // which we implement using the += operator and an accumulator like so:
      //
      dist += 0.5 * (v[i] + v[i - stepsize]) * (t[i] - t[i - stepsize]);
    }
    // round to 3 digits
    ((dist * 1000.0).round() / 1000.0) as f64
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

    let p_manifold_scrut = lap.channel("pManifoldScrut").unwrap();
    assert_eq!("pManifoldScrut", p_manifold_scrut.name());
    assert_eq!(100.0, p_manifold_scrut.frequency());

    assert_eq!(100.0, lap.max_frequency());
    assert_eq!(5326.123, lap.distance());
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
