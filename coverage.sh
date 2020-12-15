#!/bin/sh

# grcov and lcov must be installed for this to run:
#
# $ cargo install grcov
# $ sudo apt install lcov  # or what the equivalent on your platform is

# set flags for coverage
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
export RUSTDOCFLAGS="-Cpanic=abort"

# build and run tests using these two commands in every component directory
cargo build
cargo test --workspace --verbose -- --test-threads=1

# use grcov to generate report info
mkdir coverage
grcov -s . --llvm --branch --ignore-not-existing \
      -o coverage/full.info ./target/debug

# filter the report using lcov
lcov --extract coverage/full.info \
     "babylon/src/*"              \
     "carrier/src/*"              \
     "dock/src/*"                 \
     "util/src/*"                 \
     -o coverage/dock.info

# generate report for GitLab CI
lcov --list coverage/dock.info

# finally, generate HTML
genhtml --show-details --highlight --ignore-errors source --legend \
        -o ./coverage/html ./coverage/dock.info
