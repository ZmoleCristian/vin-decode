use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::data::{BodyRow, EngineRow, EuModelRow, ModelRow};
use crate::maps::{FstMap, FstSet, data_dir};
use crate::types::{BodyType, DriveType, FuelType, Transmission};

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
    eu_body: Option<FstMap<BodyRow>>,
    /// Lazily-built model → makes reverse index, cached for the Catalog's life.
    model_to_makes: OnceLock<HashMap<String, Vec<String>>>,
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
        let eu_body = if dir.join("eu_body.fst").exists() {
            Some(FstMap::open(dir)?)
        } else {
            None
        };
        Ok(Catalog {
            makes: FstSet::open(&dir.join("makes.fst"))?,
            make_models: FstMap::open(dir)?,
            eu_brand_models,
            eu_engines,
            eu_body,
            model_to_makes: OnceLock::new(),
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

    /// Reverse lookup: every make (uppercase) that lists `model`, matched
    /// case-insensitively. Merges the vPIC and EU/global indices.
    ///
    /// The inverse index is built once on first call and cached for the
    /// Catalog's lifetime, so consumers don't rebuild a model → make map per
    /// boot. Returns an empty vec for an unknown model.
    pub fn make_for_model(&self, model: &str) -> Vec<String> {
        self.model_to_makes
            .get_or_init(|| self.build_model_index())
            .get(&model.to_ascii_uppercase())
            .cloned()
            .unwrap_or_default()
    }

    fn build_model_index(&self) -> HashMap<String, Vec<String>> {
        let mut idx: HashMap<String, Vec<String>> = HashMap::new();
        for make in self.make_models.keys() {
            if let Some(rows) = self.make_models.get(&make) {
                for r in rows {
                    idx.entry(r.name.to_ascii_uppercase())
                        .or_default()
                        .push(make.clone());
                }
            }
        }
        if let Some(eu) = &self.eu_brand_models {
            for make in eu.keys() {
                if let Some(rows) = eu.get(&make) {
                    for r in rows {
                        idx.entry(r.name.to_ascii_uppercase())
                            .or_default()
                            .push(make.clone());
                    }
                }
            }
        }
        for makes in idx.values_mut() {
            makes.sort();
            makes.dedup();
        }
        idx
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

    /// EU type-approval bodywork types known for `(make, model)`.
    ///
    /// `make` is normalised the same way as [`Catalog::eu_models_for`] /
    /// [`Catalog::models_for_make`]; `model` is matched case-insensitively
    /// against the canonical uppercase model key. Returns an empty vec when the
    /// pair is unknown or the optional `eu_body` map is not embedded.
    pub fn body_for(&self, make: &str, model: &str) -> Vec<BodyType> {
        let key = crate::decoder::normalize_make(make);
        let model_key = model.to_ascii_uppercase();
        self.eu_body
            .as_ref()
            .and_then(|m| m.get(&key))
            .unwrap_or_default()
            .into_iter()
            .find(|r| r.model == model_key)
            .map(|r| r.bodies)
            .unwrap_or_default()
    }

    /// Static list of every standard [`BodyType`] variant (EU type-approval
    /// bodywork codes), useful for typed dropdowns. Excludes
    /// [`BodyType::Unknown`], which is the absence of a code, not a type.
    pub fn body_types() -> &'static [BodyType] {
        crate::types::BODY_TYPES
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

    /// Static list of every [`DriveType`] variant — useful for typed dropdowns.
    pub fn drive_types() -> &'static [DriveType] {
        &[
            DriveType::Fwd,
            DriveType::Rwd,
            DriveType::Awd,
            DriveType::FourWheelDrive,
            DriveType::Other,
        ]
    }

    /// Static list of every [`Transmission`] variant — useful for typed dropdowns.
    pub fn transmissions() -> &'static [Transmission] {
        &[
            Transmission::Manual,
            Transmission::Automatic,
            Transmission::DualClutch,
            Transmission::Cvt,
            Transmission::SingleSpeed,
            Transmission::AutomatedManual,
            Transmission::Other,
        ]
    }
}
