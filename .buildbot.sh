#! /bin/sh

set -e

export CARGO_HOME="`pwd`/.cargo"
export RUSTUP_HOME="`pwd`/.rustup"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh
sh rustup.sh --default-host x86_64-unknown-linux-gnu --default-toolchain nightly -y --no-modify-path

export PATH=`pwd`/.cargo/bin/:$PATH

# Sometimes rustfmt is so broken that there is no way to install it at all.
# Rather than refusing to merge, we just can't rust rustfmt at such a point.
rustup component add --toolchain nightly rustfmt-preview \
    || cargo +nightly install --force rustfmt-nightly \
    || true
rustfmt=0
cargo fmt 2>&1 | grep "not installed for the toolchain" > /dev/null || rustfmt=1
if [ $rustfmt -eq 1 ]; then
    cargo +nightly fmt --all -- --check
fi

cargo check
