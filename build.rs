// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use std::{env, fs, path::Path};


fn main() {
  #[rustfmt::skip]
  let project_dir = env::var(
    "CARGO_MANIFEST_DIR"
  ).expect("unable to read CARGO_MANIFEST_DIR env variable");

  #[rustfmt::skip]
  let out_dir = env::var(
    "OUT_DIR"
  ).expect("unable to read OUT_DIR env variable");

  let lib_src_path = format!("{}/aim", project_dir);
  let lib_dst_path = format!("{}/lib", out_dir);

  if !Path::new(&lib_dst_path).exists() {
    fs::create_dir(&lib_dst_path).expect("unable to create lib dir");
  }

  let files = fs::read_dir(&lib_src_path).expect("unable to read aim dir");
  for file in files {
    let src_path = file.expect("could not read file").path();
    let dst_path = format!("{}/{}",
                           &lib_dst_path,
                           src_path.file_name().unwrap().to_str().unwrap());

    fs::copy(src_path, dst_path).expect("unable to copy libs to target dir");
  }

  println!(r"cargo:rustc-link-search=all={}/lib", out_dir);

  #[cfg(target_family = "unix")]
  {
    println!(r"cargo:rustc-link-lib=xdrk-x86_64");
    println!(r"cargo:rustc-link-lib=xml2");
  }

  #[cfg(target_family = "windows")]
  {
    println!(r"cargo:rustc-link-lib=dylib=libxdrk-x86_64");
  }
}
