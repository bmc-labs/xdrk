// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

//! `xrdkrs` is a Rust wrapper around the shared library to access data in XRK
//! or DRK format. Such data is recorded by devices from _AiM Tech Srl_, a
//! company focused on data logging products for the motor racing segment.
//!
//! The formats themselves are proprietary, but the data they contain is fairly
//! straight forward:
//!
//! - some meta information about the library itself
//! - information with regard to the _laps_ contained, where a _lap_ is a data
//!   segment within a list of segments produced by splitting the data because
//!   of some higher level information (e.g. _arriving on the finishing line_)
//! - time series data of logged sensory measurements, where one sample is
//!   expressed as a timestamp and a corresponding measurement
//!
//! This crate wraps the original library and provides a safe, Rust-idiomatic
//! interface to its functionality. Aside from the raw API provided by the
//! library, higher level functions for retrieving data are provided, as well
//! as machinery to synchronize the raw data into matching time series.

mod service;
mod storage;
mod xdrkbindings;
mod xdrkfile;

pub use storage::{Channel, ChannelData, LapInfo};
pub use xdrkfile::XdrkFile;
