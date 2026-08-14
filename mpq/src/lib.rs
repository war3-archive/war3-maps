//! A library for reading MPQ archives

#![allow(clippy::unreadable_literal)]

mod archive;
pub mod carve;
mod chain;
mod compression;
mod crypt;
mod salvage;

pub use crate::archive::{Archive, File};
pub use crate::chain::Chain;
pub use crate::salvage::SalvagedMember;
