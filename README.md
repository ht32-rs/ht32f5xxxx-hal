# [Documentation](https://docs.rs/ht32f5xxxx-hal)

ht32f5xxxx-hal
=============

[![docs.rs](https://docs.rs/ht32f5xxxx-hal/badge.svg)](https://docs.rs/ht32f5xxxx-hal)
[![Crates.io](https://img.shields.io/crates/v/ht32f5xxxx-hal.svg)](https://crates.io/crates/ht32f5xxxx-hal)
![Continuous integration](https://github.com/ht32-rs/ht32f5xxxx-hal/workflows/Continuous%20integration/badge.svg)

This crate implements the embedded-hal abstractions for the Holtek HT32F5xxxx chip family. Its original purpose is to serve
as a platform for porting the [anne-key](https://github.com/ah-/anne-key) project on to the Anne Pro 2 PCB which uses a
HT32F52342 MCU.

Collaboration on this crate is highly welcome, as are pull requests!

## Disclaimer
Every piece of code in this crate has only been tested on the HT32F52352 it might be able to work out of the box on other
members of the HT32F5xxxx family (which is very likely as Holtek only provides one firmware package for the entire family) but
has not been tested or ever run there.

## Getting started
The `examples` folder contains several example programs. To compile
them, one must specify the target device as cargo feature:
```
$ cargo build --features=ht32f52342_52,rt
```

To use this crate as a dependency in a standalone project the
target device feature must be specified in the `Cargo.toml` file:
```
[dependencies]
cortex-m = "0.6.2"
cortex-m-rt = "0.6.12"
ht32f5xxxx-hal = {version = "0.0.1", features = ["ht32f52342_52","rt"]}
```
