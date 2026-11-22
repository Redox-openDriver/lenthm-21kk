#![cfg_attr(not(feature = "diary"), no_std)]

extern crate alloc;

#[cfg(feature = "diary")]
pub mod diary;
pub mod ecmb;
