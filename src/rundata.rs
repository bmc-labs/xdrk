// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Jonas Reitemeyer <jonas@bmc-labs.com>
//   Florian Eich <florian@bmc-labs.com>

use super::{lap::Lap, xdrkfile::XdrkFile};

use anyhow::Result;
use getset::Getters;
use std::path::Path;


/// Root Object for holding all the Data, which is grouped per lap
#[derive(Debug, PartialEq, Getters)]
pub struct RunData {
  laps:     Vec<Lap>,
  channels: Vec<String>,
}

impl RunData {
  pub fn new(path: &Path) -> Result<Self> {
    println!("{:#?}", path);
    let xdrk = XdrkFile::load(path)?;
    println!("{:#?}", xdrk);
    let laps_count = xdrk.laps_count()?;
    println!("{:#?}", laps_count);
    let channels_count = xdrk.channels_count()?;
    println!("{:#?}", channels_count);

    let mut laps: Vec<Lap> = Vec::new();
    for i in 0..laps_count {
      let lap_info = xdrk.lap_info(i)?;
      laps.push(Lap::new(lap_info, xdrk.lap_data(i)?));
    }

    let mut channels: Vec<String> = Vec::new();
    for i in 0..channels_count {
      channels.push(xdrk.channel_name(i)?);
    }

    Ok(Self { laps, channels })
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  // use pretty_assertions::assert_eq;
  // use std::fs;

  static XRK_PATH: &str =
    "./testdata/WT-20_E05-ARA_Q3_AU-RS3-R5-S-S_017_a_1220.xrk";

  #[test]
  fn rundata_test() {
    let _run_data = RunData::new(Path::new(XRK_PATH));
    // println!("{:#?}", _run_data);
  }
}
