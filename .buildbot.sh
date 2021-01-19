#! /bin/sh

set -e

export CARGO_HOME="`pwd`/.cargo"
export RUSTUP_HOME="`pwd`/.rustup"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh
sh rustup.sh --default-host x86_64-unknown-linux-gnu \
    --default-toolchain nightly \
    --no-modify-path \
    --profile minimal \
    -y
export PATH=`pwd`/.cargo/bin/:$PATH
cargo check

rustup toolchain install nightly --allow-downgrade --component rustfmt
cargo +nightly fmt --all -- --check

# Build and test with rustgc
git clone https://github.com/softdevteam/rustgc
mkdir -p rustgc/build/rustgc
(cd rustgc && ./x.py build --config ../.buildbot.config.toml)

rustup toolchain link rustgc rustgc/build/x86_64-unknown-linux-gnu/stage1

cargo clean

cargo +rustgc test --features "rustgc" -- --test-threads=1
