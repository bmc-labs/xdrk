// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::{Lap, XdrkFile};
use anyhow::Result;
use chrono::NaiveDateTime;
use getset::Getters;
use std::path::Path;


/// Holds all information and data corresponding to one run.
#[derive(Debug, PartialEq, Getters)]
#[getset(get = "pub")]
pub struct Run {
  championship:  String, // "WT-20_E05-ARA"
  track:         String, // "ARA_1-0-0"
  venue_type:    String, // "Q3"
  vehicle:       String, // "HY-i30N-C4-X-S"
  racer:         String, // "030"
  datetime:      NaiveDateTime,
  channel_names: Vec<String>,
  laps:          Vec<Lap>,
}

impl Run {
  pub fn new(path: &str) -> Result<Self> {
    let xdrk = XdrkFile::load(Path::new(path))?;

    Ok(Self { championship:  xdrk.championship()?,
              track:         xdrk.track()?,
              venue_type:    xdrk.venue_type()?,
              vehicle:       xdrk.vehicle()?,
              racer:         xdrk.racer()?,
              datetime:      xdrk.datetime()?,
              channel_names: xdrk.channel_names()?,
              laps:          xdrk.all_laps()?, })
  }

  pub fn number_of_channels(&self) -> usize {
    self.channel_names.len()
  }

  pub fn number_of_laps(&self) -> usize {
    self.laps.len()
  }

  pub fn max_frequency(&self) -> f64 {
    if self.laps.is_empty() {
      return 0.0;
    }
    let frequency = self.laps[0].max_frequency();
    assert!(self.laps.iter().all(|lap| frequency == lap.max_frequency()));
    frequency
  }

  pub fn yield_laps(self) -> Vec<Lap> {
    self.laps
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::{assert_eq, assert_ne};


  const XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn rundata_test() {
    let run = Run::new(XRK_PATH).unwrap();

    assert_eq!("WT-20", run.championship());
    assert_eq!("ARA_1-0-0", run.track());
    assert_eq!("Q3", run.venue_type());
    assert_eq!("AU-RS3-R5-S-S", run.vehicle());
    assert_eq!("017", run.racer());

    assert_eq!("2020-11-14 16:49:39", run.datetime().to_string());

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
    assert_eq!(&channel_names, run.channel_names());
    for lap in run.laps() {
      assert_eq!(channel_names, lap.channel_names());
    }

    assert_eq!(51, run.number_of_channels());
    assert_eq!(4, run.number_of_laps());
    assert_eq!(100.0, run.max_frequency());

    let sec_path = "./testdata/WT-20_E05-ARA_Q2_AU-RS3-R5-S-S_016_a_1139.xrk";
    assert_ne!(run, Run::new(sec_path).unwrap());
  }
}
