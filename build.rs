// Copyright 2020 bmc::labs Gmbh. All rights reserved.
//
// Author: Florian Eich <florian@bmc-labs.com>

use std::env;


fn main() {
  let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  println!("cargo:rustc-link-search={}/aim", project_dir);

  #[cfg(target_family = "unix")]
  {
    println!("cargo:rustc-link-lib=xdrk-x86_64");
    println!("cargo:rustc-link-lib=xml2");
  }

  #[cfg(target_family = "windows")]
  {
    println!("cargo:rustc-link-lib=dylib=libxdrk-x86_64");
  }
}
