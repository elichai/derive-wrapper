# Derive-Wrapper
[![Build Status](https://travis-ci.org/elichai/derive-wrapper.svg?branch=master)](https://travis-ci.org/elichai/derive-wrapper)
[![Latest version](https://img.shields.io/crates/v/derive-wrapper.svg)](https://crates.io/crates/derive-wrapper)
![License](https://img.shields.io/crates/l/derive-wrapper.svg)

A custom derive macro helper that let's you easily derive traits for wrapper types.
## Examples:
```rust
#[derive(Debug, Default, Index, AsRef, LowerHexIter)]
struct Array32([u8; 32]);

#[derive(Debug, Default, LowerHex)]
struct Flag(i32);

#[derive(Debug, Index, LowerHexIter)]
struct Hi {
    #[wrap]
    a: [u8; 32],
    b: Flag,
}

#[derive(Debug, Display, From, Error)]
#[display_from(Debug)]
struct Printer<T: std::fmt::Debug>(T);

#[derive(Default, LowerHex, Display)]
#[display_from(LowerHex)]
#[wrap = "two"]
struct Big {
    one: Array32,
    two: Hi,
}

#[derive(From)]
enum MyEnum<T> {
    #[derive_from]
    First(u8),
    #[derive_from]
    Second(Array32),
    Third,
    #[derive_from]
    Fourth {
        other: Vec<u8>,
    },
    #[derive_from]
    Fifth(PhantomData<T>),
}
```