#![no_std]

#[cfg(any(feature = "lcg", feature = "chacha"))]
use foundation::ops::RandomOps;

pub mod chacha;
pub mod lcg;

#[cfg(feature = "lcg")]
pub const RNG_OPS: RandomOps = RandomOps {
    init: lcg::init,
    fill_bytes: lcg::fill_bytes,
};

#[cfg(feature = "chacha")]
pub const RNG_OPS: RandomOps = RandomOps {
    init: chacha::init,
    fill_bytes: chacha::fill_bytes,
};

#[cfg(test)]
mod tests;
