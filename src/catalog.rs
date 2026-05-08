use std::path::Path;

use crate::data::{EngineRow, EuModelRow, ModelRow};
use crate::maps::{FstMap, FstSet, data_dir};
use crate::types::{BodyClass, FuelType};

/// Read-only catalog of every make and model present in the lookup data.
///
/// Useful for populating dropdowns, validators, or schema-typed enum columns
/// in webapps. Combines the vPIC pattern-derived index (`make_models`) with the
/// EU/global rip catalog (`eu_brand_models`, `eu_engines`) for non-US coverage.
pub struct Catalog {
    makes: FstSet,
    make_models: FstMap<ModelRow>,
    eu_brand_models: Option<FstMap<EuModelRow>>,
    eu_engines: Option<FstMap<EngineRow>>,
}

impl Catalog {
    /// Open the catalog using the default data directory (see [`crate::Decoder::new`]).
    pub fn new() -> crate::Result<Self> {
        let dir = data_dir();
        #[cfg(feature = "embedded")]
        crate::embedded::ensure_installed(&dir).ok();
        Self::open(&dir)
    }

    /// Open the catalog against an explicit data directory. EU rip tables are
    /// optional — they're only loaded if the corresponding `.fst` files exist.
    pub fn open(dir: &Path) -> crate::Result<Self> {
        let eu_brand_models = if dir.join("eu_brand_models.fst").exists() {
            Some(FstMap::open(dir)?)
        } else {
            None
        };
        let eu_engines = if dir.join("eu_engines.fst").exists() {
            Some(FstMap::open(dir)?)
        } else {
            None
        };
        Ok(Catalog {
            makes: FstSet::open(&dir.join("makes.fst"))?,
            make_models: FstMap::open(dir)?,
            eu_brand_models,
            eu_engines,
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
    ///
    /// Merges results from the vPIC pattern-derived index and the EU/global rip
    /// catalog, deduped and sorted.
    pub fn models_for_make(&self, make: &str) -> Vec<String> {
        let key = crate::decoder::normalize_make(make);
        let mut models: Vec<String> = self
            .make_models
            .get(&key)
            .map(|rows| rows.into_iter().map(|r| r.name).collect())
            .unwrap_or_default();
        if let Some(eu) = &self.eu_brand_models {
            if let Some(rows) = eu.get(&key) {
                for r in rows {
                    models.push(r.name);
                }
            }
        }
        models.sort();
        models.dedup();
        models
    }

    /// EU/global rip listing of models for the given brand, with year ranges.
    /// Returns an empty vec when the brand is unknown or the EU catalog is not
    /// embedded.
    pub fn eu_models_for(&self, brand: &str) -> Vec<EuModelRow> {
        let key = crate::decoder::normalize_make(brand);
        self.eu_brand_models
            .as_ref()
            .and_then(|m| m.get(&key))
            .unwrap_or_default()
    }

    /// Engine variants known for the given brand. Filter by `model` in user
    /// code, or use [`Catalog::engines_for`].
    pub fn engines_for_brand(&self, brand: &str) -> Vec<EngineRow> {
        let key = crate::decoder::normalize_make(brand);
        self.eu_engines
            .as_ref()
            .and_then(|m| m.get(&key))
            .unwrap_or_default()
    }

    /// Engine variants known for `(brand, model)`. Both args are matched
    /// case-insensitively against the canonical uppercase keys.
    pub fn engines_for(&self, brand: &str, model: &str) -> Vec<EngineRow> {
        let model_key = model.to_ascii_uppercase();
        self.engines_for_brand(brand)
            .into_iter()
            .filter(|r| r.model == model_key)
            .collect()
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
