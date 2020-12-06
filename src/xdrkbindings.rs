// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use std::os::raw::{c_char, c_int};


/// Binding to C tm struct storing datetime info (defined in `time.h`)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub struct tm {
  pub tm_sec:   c_int, // seconds, range 0 to 59
  pub tm_min:   c_int, // minutes, range 0 to 59
  pub tm_hour:  c_int, // hours, range 0 to 23
  pub tm_mday:  c_int, // day of the month, range 1 to 31
  pub tm_mon:   c_int, // month, range 0 to 11
  pub tm_year:  c_int, // number of years since 1900
  pub tm_wday:  c_int, // day of the week, range 0 to 6
  pub tm_yday:  c_int, // day in the year, range 0 to 365
  pub tm_isdst: c_int, // daylight saving time
}


#[allow(dead_code)]
#[doc(hidden)]
extern "C" {
  // FILE OPENING / CLOSING FUNCTIONS -------------------------------------- //
  //
  /// Open a drk/xrk file
  ///
  /// # Arguments
  /// - `full_path_name`: full path to the file to be opened as a C string
  ///
  /// # Returns
  /// - on success, the (positive) internal index of the file
  /// - `0` if the file is opened but can't be parsed
  /// - on error, a negative value
  pub fn open_file(full_path_name: *const c_char) -> c_int;

  /// Close a drk/xrk file by path
  ///
  /// # Arguments
  /// - `full_path_name`: full path to the file to be closed as a C string
  ///
  /// # Returns
  /// - on success, the (positive) internal index of the file
  /// - on error, a negative value
  pub fn close_file_n(full_path_name: *const c_char) -> c_int;

  /// Close a drk/xrk file by internal index
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, the (positive) internal index of the file
  /// - on error, a negative value
  pub fn close_file_i(idx: c_int) -> c_int;
  // ----------------------------------------------------------------------- //

  // SESSION INFORMATION FUNCTIONS ----------------------------------------- //
  //
  /// Get vehicle info
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C string with the vehicle info
  /// - on error, `NULL`
  pub fn get_vehicle_name(idx: c_int) -> *const c_char;

  /// Get track info
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C string with the track info
  /// - on error, `NULL`
  pub fn get_track_name(idx: c_int) -> *const c_char;

  /// Get racer info
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C string with the racer info
  /// - on error, `NULL`
  pub fn get_racer_name(idx: c_int) -> *const c_char;

  /// Get championship info
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C string with the championship info
  /// - on error, `NULL`
  pub fn get_championship_name(idx: c_int) -> *const c_char;

  /// Get venue type info
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C string with the venue type info
  /// - on error, `NULL`
  pub fn get_venue_type_name(idx: c_int) -> *const c_char;

  /// Get session date and time
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, a C pointer to a `tm` struct
  /// - on error, `NULL`
  pub fn get_date_and_time(idx: c_int) -> *const tm;

  /// Get number of laps contained in a drk/xrk file
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, the number of laps
  /// - `0` if the file contains no laps (theoretically impossible)
  /// - on error, a negative value
  pub fn get_laps_count(idx: c_int) -> c_int;

  /// Get lap info
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: index of the lap in question
  /// - `pstart`: pointer to a `mut f64` where the start time (in seconds since
  ///   start of the session) is stored
  /// - `pduration`: pointer to a `mut f64` where the lap time is stored
  ///
  /// # Returns
  /// - `1` on success
  /// - `0` if the file contains no laps (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_info(idxf: c_int,
                      idxl: c_int,
                      pstart: *mut f64,
                      pduration: *mut f64)
                      -> c_int;
  // ----------------------------------------------------------------------- //


  // CHANNEL INFORMATION FUNCTIONS ----------------------------------------- //
  //
  /// Get number of channels contained in a drk/xrk file
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on success, the number of channels
  /// - `0` if the file contains no channels (theoretically impossible)
  /// - on error, a negative value
  pub fn get_channels_count(idx: c_int) -> c_int;

  /// Get channel name
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel name
  /// - on error, `NULL`
  pub fn get_channel_name(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get channel units
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel units
  /// - on error, `NULL`
  pub fn get_channel_units(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get number of datapoints in channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_channel_samples_count(idxf: c_int, idxc: c_int) -> c_int;
  /// Get datapoints in channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_channel_samples(idxf: c_int,
                             idxc: c_int,
                             ptimes: *mut f64,
                             pvalues: *mut f64,
                             cnt: c_int)
                             -> c_int;

  /// Get number of datapoints in channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel in the lap
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_channel_samples_count(idxf: c_int,
                                       idxl: c_int,
                                       idxc: c_int)
                                       -> c_int;

  /// Get datapoints in channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_channel_samples(idxf: c_int,
                                 idxl: c_int,
                                 idxc: c_int,
                                 ptimes: *mut f64,
                                 pvalues: *mut f64,
                                 cnt: c_int)
                                 -> c_int;
  // ----------------------------------------------------------------------- //


  // GPS INFORMATION FUNCTIONS --------------------------------------------- //
  //
  // GPS channels are the same channels added to AiM drk files in RS2Analysis,
  // those that consider vehicle dynamics assuming that the vehicle is
  // constantly aligned to the trajectory.
  //
  /// Get GPS channels count of a xrk file
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on sucess, the number of GPS channels available
  /// - `0` if the file has no GPS channels (theoretically not possible)
  /// - on error, a negative value
  pub fn get_GPS_channels_count(idx: c_int) -> c_int;

  /// Get GPS channel name
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel name
  /// - on error, `NULL`
  pub fn get_GPS_channel_name(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get GPS channel units
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel units
  /// - on error, `NULL`
  pub fn get_GPS_channel_units(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get number of datapoints in GPS channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_GPS_channel_samples_count(idxf: c_int, idxc: c_int) -> c_int;

  /// Get datapoints in GPS channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_GPS_channel_samples(idxf: c_int,
                                 idxc: c_int,
                                 ptimes: *mut f64,
                                 pvalues: *mut f64,
                                 cnt: c_int)
                                 -> c_int;

  /// Get number of datapoints in GPS channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel in the lap
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_GPS_channel_samples_count(idxf: c_int,
                                           idxl: c_int,
                                           idxc: c_int)
                                           -> c_int;

  /// Get datapoints in GPS channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_GPS_channel_samples(idxf: c_int,
                                     idxl: c_int,
                                     idxc: c_int,
                                     ptimes: *mut f64,
                                     pvalues: *mut f64,
                                     cnt: c_int)
                                     -> c_int;

  /// Get GPS raw channels count of a xrk file
  ///
  /// # Arguments
  /// - `idx`: the internal file index returned by the `open_file` function
  ///
  /// # Returns
  /// - on sucess, the number of GPS raw channels available
  /// - `0` if the file has no GPS raw channels (theoretically not possible)
  /// - on error, a negative value
  pub fn get_GPS_raw_channels_count(idx: c_int) -> c_int;

  /// Get GPS raw channel name
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel name
  /// - on error, `NULL`
  pub fn get_GPS_raw_channel_name(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get GPS raw channel units
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, a C string with the channel units
  /// - on error, `NULL`
  pub fn get_GPS_raw_channel_units(idxf: c_int, idxc: c_int) -> *const c_char;

  /// Get number of datapoints in GPS raw channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_GPS_raw_channel_samples_count(idxf: c_int, idxc: c_int) -> c_int;

  /// Get datapoints in GPS raw channel
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_GPS_raw_channel_samples(idxf: c_int,
                                     idxc: c_int,
                                     ptimes: *mut f64,
                                     pvalues: *mut f64,
                                     cnt: c_int)
                                     -> c_int;

  /// Get number of datapoints in GPS raw channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel in the lap
  /// - `0` if the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_GPS_raw_channel_samples_count(idxf: c_int,
                                               idxl: c_int,
                                               idxc: c_int)
                                               -> c_int;

  /// Get datapoints in GPS raw channel in a given lap
  ///
  /// # Arguments
  /// - `idxf`: the internal file index returned by the `open_file` function
  /// - `idxl`: the lap index
  /// - `idxc`: the channel index
  /// - `ptimes`: a pointer to **a buffer** of `mut f64` where timestamps of
  /// datapoints are stored
  /// - `pvalues`: a pointer to **a buffer** of `mut f64` where datapoints are
  /// stored
  /// - `cnt`: the number of datapoints to be read (find using the
  /// `get_channel_samples_count` function)
  ///
  /// # Returns
  /// - on success, the number of datapoints in the channel
  /// - `0` if the `cnt` argument does not match the number of datapoints OR if
  /// the channel contains no datapoints (theoretically impossible)
  /// - on error, a negative value
  pub fn get_lap_GPS_raw_channel_samples(idxf: c_int,
                                         idxl: c_int,
                                         idxc: c_int,
                                         ptimes: *mut f64,
                                         pvalues: *mut f64,
                                         cnt: c_int)
                                         -> c_int;

  // META FUNCTIONS -------------------------------------------------------- //
  /// Returns the compile date of this library as a C string
  pub fn get_library_date() -> *const c_char;

  /// Returns the compile time of this library as a C string
  pub fn get_library_time() -> *const c_char;
// ----------------------------------------------------------------------- //

}
