use super::channel::Channel;
use getset::{CopyGetters, Getters};

/// Hold all channels of a single lap
#[derive(Debug, PartialEq, Getters)]
pub struct Lap {
  channels: Vec<Channel>,
  info:     LapInfo,
}

impl Lap {
  pub fn new(info: LapInfo, channels: Vec<Channel>) -> Self {
    Self { info, channels }
  }
}

/// Stores the start time within the recording and the duration of a lap.
#[derive(Debug, PartialEq, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct LapInfo {
  start:    f64,
  duration: f64,
}

impl LapInfo {
  pub fn new(start: f64, duration: f64) -> Self {
    Self { start, duration }
  }
}
