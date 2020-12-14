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
  championship:  String,
  track:         String,
  venue_type:    String,
  vehicle:       String,
  racer:         String,
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
}


#[cfg(test)]
mod tests {
  use super::*;


  static XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn rundata_test() {
    let _run_data = RunData::new(XRK_PATH).unwrap();
  }
}
