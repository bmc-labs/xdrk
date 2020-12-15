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
pub struct RunData {
  championship:  String, // "WT-20_E05"
  track:         String, // "ARA"
  venue_type:    String, // "Q3"
  vehicle:       String, // "HY-i30N-C4-X-S"
  racer:         String, // "030"
  datetime:      NaiveDateTime,
  channel_names: Vec<String>,
  laps:          Vec<Lap>,
}

impl RunData {
  pub fn new(path: &str) -> Result<Self> {
    let xdrk = XdrkFile::load(Path::new(path))?;

    Ok(Self { championship:  xdrk.championship_name()?,
              track:         xdrk.track_name()?,
              venue_type:    xdrk.venue_type_name()?,
              vehicle:       xdrk.vehicle_name()?,
              racer:         xdrk.racer_name()?,
              datetime:      xdrk.date_time()?,
              channel_names: xdrk.channel_names()?,
              laps:          xdrk.laps()?, })
  }

  pub fn number_of_channels(&self) -> usize {
    self.channel_names.len()
  }

  pub fn number_of_laps(&self) -> usize {
    self.laps.len()
  }

  pub fn frequency(&self) -> usize {
    if self.laps.is_empty() {
      return 0;
    }
    let frequency = self.laps[0].frequency();
    assert!(self.laps.iter().all(|lap| frequency == lap.frequency()));

    frequency
  }
}


#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;


  const XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn rundata_test() {
    let run_data = RunData::new(XRK_PATH).unwrap();

    assert_eq!("WT-20", run_data.championship());
    assert_eq!("ARA_1-0-0", run_data.track());
    assert_eq!("Q3", run_data.venue_type());
    assert_eq!("AU-RS3-R5-S-S", run_data.vehicle());
    assert_eq!("017", run_data.racer());

    assert_eq!("2020-11-14 16:49:39", run_data.datetime().to_string());

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
                                   "swGearDOWN"];
    assert_eq!(&channel_names, run_data.channel_names());
    for lap in run_data.laps() {
      assert_eq!(channel_names, lap.channel_names());
    }

    assert_eq!(40, run_data.number_of_channels());
    assert_eq!(4, run_data.number_of_laps());
  }
}
