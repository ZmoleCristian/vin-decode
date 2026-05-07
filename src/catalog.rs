use std::path::Path;

use crate::data::ModelRow;
use crate::maps::{FstMap, FstSet, data_dir};
use crate::types::{BodyClass, FuelType};

/// Read-only catalog of every make and model present in the lookup data.
///
/// Useful for populating dropdowns, validators, or schema-typed enum columns
/// in webapps. All listings come straight from the embedded vPIC data.
pub struct Catalog {
    makes: FstSet,
    make_models: FstMap<ModelRow>,
}

impl Catalog {
    /// Open the catalog using the default data directory (see [`crate::Decoder::new`]).
    pub fn new() -> crate::Result<Self> {
        let dir = data_dir();
        #[cfg(feature = "embedded")]
        crate::embedded::ensure_installed(&dir).ok();
        Self::open(&dir)
    }

    /// Open the catalog against an explicit data directory.
    pub fn open(dir: &Path) -> crate::Result<Self> {
        Ok(Catalog {
            makes: FstSet::open(&dir.join("makes.fst"))?,
            make_models: FstMap::open(dir)?,
        })
    }

    /// Sorted list of every make name (uppercase, deduped).
    pub fn all_makes(&self) -> Vec<String> {
        self.makes.keys()
    }

    /// Case-insensitive membership check for a make name.
    pub fn has_make(&self, make: &str) -> bool {
        self.makes.contains(&make.to_ascii_uppercase())
    }

    /// Total number of distinct makes.
    pub fn make_count(&self) -> u64 {
        self.makes.len()
    }

    /// Sorted list of model names known for the given make (case-insensitive lookup).
    pub fn models_for_make(&self, make: &str) -> Vec<String> {
        let key = make.to_ascii_uppercase();
        self.make_models
            .get(&key)
            .map(|rows| rows.into_iter().map(|r| r.name).collect())
            .unwrap_or_default()
    }

    /// Static list of every [`BodyClass`] variant — useful for typed dropdowns.
    pub fn body_classes() -> &'static [BodyClass] {
        &[
            BodyClass::Sedan,
            BodyClass::Coupe,
            BodyClass::Hatchback,
            BodyClass::Wagon,
            BodyClass::Convertible,
            BodyClass::Suv,
            BodyClass::Crossover,
            BodyClass::Pickup,
            BodyClass::Van,
            BodyClass::Minivan,
            BodyClass::Bus,
            BodyClass::Truck,
            BodyClass::Motorcycle,
            BodyClass::Trailer,
            BodyClass::Incomplete,
            BodyClass::Other,
        ]
    }

    /// Static list of every [`FuelType`] variant — useful for typed dropdowns.
    pub fn fuel_types() -> &'static [FuelType] {
        &[
            FuelType::Gasoline,
            FuelType::Diesel,
            FuelType::Electric,
            FuelType::Hybrid,
            FuelType::PluginHybrid,
            FuelType::Ethanol,
            FuelType::FlexFuel,
            FuelType::Cng,
            FuelType::Lng,
            FuelType::Lpg,
            FuelType::Hydrogen,
            FuelType::FuelCell,
            FuelType::Methanol,
            FuelType::NaturalGas,
            FuelType::Other,
        ]
    }
}
