//! Auto-updating VIN decoder backed by the NHTSA vPIC database.
//!
//! This crate ships pre-built FST/rkyv lookup maps that get refreshed monthly
//! from the official NHTSA vPIC dump via CI. End users get fresh decoder data
//! without ever touching the network at runtime.
//!
//! # Example
//!
//! ```no_run
//! use vin_decode::Decoder;
//!
//! let dec = Decoder::new()?;
//! let v = dec.decode("1HGCM82633A004352")?;
//! assert_eq!(v.make.as_deref(), Some("Honda"));
//! # Ok::<(), vin_decode::Error>(())
//! ```

#![deny(missing_docs)]

#[cfg(feature = "build")]
pub mod build;

pub mod data;

pub(crate) mod maps;

mod catalog;
mod check_digit;
mod country;
mod decoder;
mod element;
mod error;
mod pattern;
mod types;
mod wmi;
mod year;

#[cfg(feature = "embedded")]
mod embedded;

pub use catalog::Catalog;
pub use country::country_from_code;
pub use data::{EngineRow, EuModelRow};
pub use decoder::Decoder;
pub use error::{Error, Result};
pub use types::{BodyClass, FuelType, Vehicle, Vin};
pub use wmi::region as region_from_code;

#[cfg(feature = "build")]
#[doc(hidden)]
pub use maps::{FstMap, FstSet};

use static_assertions::assert_impl_all;
assert_impl_all!(Decoder: Send, Sync);
assert_impl_all!(Catalog: Send, Sync);
assert_impl_all!(Vehicle: Send, Sync);
assert_impl_all!(Vin: Send, Sync);
