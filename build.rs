// Copyright 2021 bmc::labs Gmbh. All rights reserved.
//
// Authors:
//   Florian Eich <florian@bmc-labs.com>
//   Jonas Reitemeyer <alumni@bmc-labs.com>

use std::{env, fs, path::Path};


fn main() {
  // because of the dynamic linking foo required to make these shared libraries
  // from AiM work on both platforms, we need to do some extra acrobatics in
  // this build script here.
  //
  // first and foremost, we must find the paths for the project and the
  // OUT_DIR, which is a cargo variable containing the location where cargo
  // will drop all the goods (binaries et al.).
  #[rustfmt::skip]
  let project_dir = env::var(
    "CARGO_MANIFEST_DIR"
  ).expect("unable to read CARGO_MANIFEST_DIR env variable");

  #[rustfmt::skip]
  let out_dir = env::var(
    "OUT_DIR"
  ).expect("unable to read OUT_DIR env variable");

  // cargo will let us specify
  //
  // (a) where to look for the libraries and stuff when we're linking, but MORE
  // IMPORTANTLY
  //
  // (b) where we want to load from at run time, but THIS ONLY IF the path we
  // provide is within the OUT_DIR
  //
  // so we must copy all the .dll, .so, .lib and whatnot to the OUT_DIR and
  // therefore we need to know each of those locations.
  let lib_src_path = format!("{}/aim", project_dir);
  let lib_dst_path = format!("{}/lib", out_dir);

  // if the destination directory doesn't yet exist, create it
  if !Path::new(&lib_dst_path).exists() {
    fs::create_dir(&lib_dst_path).expect("unable to create lib dir");
  }

  // now please copy all contents from `aim` (where the repo stores the .dll,
  // .so and so on) to the destination directory within the OUT_DIR
  let files = fs::read_dir(&lib_src_path).expect("unable to read aim dir");
  for file in files {
    let src_path = file.expect("could not read file").path();
    let dst_path = format!("{}/{}",
                           &lib_dst_path,
                           src_path.file_name().unwrap().to_str().unwrap());

    fs::copy(src_path, dst_path).expect("unable to copy libs to target dir");
  }

  // finally, tell cargo where it should be looking for the libs
  println!(r"cargo:rustc-link-search=all={}/lib", out_dir);

  // here we specify the deps for each platform. not sure why on Linux we
  // depend on libxml2 and on windows not, but not too interested either tbh
  #[cfg(target_family = "unix")]
  {
    println!(r"cargo:rustc-link-lib=xdrk-x86_64");
    println!(r"cargo:rustc-link-lib=xml2");
  }

  #[cfg(target_family = "windows")]
  {
    println!(r"cargo:rustc-link-lib=dylib=libxdrk-x86_64");
  }
  // and we're done, now we (i.e. cargo) can start doing actual work
}
