use std::collections::HashMap;
use std::path::Path;

use crate::data::{LookupRow, MakeRow, SchemaRow};
use crate::element::Element;
use crate::maps::{FstMap, data_dir};
use crate::types::{Vehicle, Vin};
use crate::{Error, check_digit, pattern, year};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// VIN decoder backed by mmap'd FST/rkyv lookup maps.
///
/// Construct via [`Decoder::new`] (uses default data dir / env var) or
/// [`Decoder::open`] (explicit path).
pub struct Decoder {
    wmi_make: FstMap<MakeRow>,
    wmi_schema: FstMap<SchemaRow>,
    schema_lookup: FstMap<LookupRow>,
}

impl Decoder {
    /// Open the decoder using the default data directory.
    ///
    /// Resolution order:
    /// 1. `VIN_DECODE_DATA_DIR` environment variable
    /// 2. `$HOME/.vin-decode-cache`
    /// 3. `./.vin-decode-cache`
    ///
    /// With the `embedded` feature, this also auto-installs bundled data into
    /// the resolved directory on first run.
    pub fn new() -> crate::Result<Self> {
        let dir = data_dir();
        #[cfg(feature = "embedded")]
        crate::embedded::ensure_installed(&dir).ok();
        Self::open(&dir).map_err(|e| match e {
            Error::MissingData(path) => Error::MissingData(format!(
                "{path} — set VIN_DECODE_DATA_DIR or install embedded data"
            )),
            other => other,
        })
    }

    /// Open the decoder against an explicit data directory.
    pub fn open(dir: &Path) -> crate::Result<Self> {
        Ok(Decoder {
            wmi_make: FstMap::open(dir)?,
            wmi_schema: FstMap::open(dir)?,
            schema_lookup: FstMap::open(dir)?,
        })
    }

    /// Decode a VIN with full validation (length, charset, check digit).
    pub fn decode(&self, raw: &str) -> crate::Result<Vehicle> {
        let vin = Vin::new(raw)?;
        check_digit::validate(&vin)?;
        Ok(self.decode_inner(vin))
    }

    /// Decode a VIN, skipping the check-digit step.
    ///
    /// Useful for VINs from non-NHTSA jurisdictions where the check digit isn't enforced.
    pub fn decode_unchecked(&self, raw: &str) -> crate::Result<Vehicle> {
        let vin = Vin::new(raw)?;
        Ok(self.decode_inner(vin))
    }

    /// Decode a slice of VINs (parallelized with `parallel` feature, sequential otherwise).
    pub fn decode_batch(&self, vins: &[&str]) -> Vec<crate::Result<Vehicle>>
    where
        Self: Sync,
    {
        #[cfg(feature = "parallel")]
        {
            vins.par_iter().map(|v| self.decode(v)).collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            vins.iter().map(|v| self.decode(v)).collect()
        }
    }

    fn decode_inner(&self, vin: Vin) -> Vehicle {
        let wmi = vin.wmi().to_string();
        let mut make_row = self
            .wmi_make
            .get(&wmi)
            .and_then(|mut rows| rows.pop());
        let make = make_row.as_ref().map(|r| r.name.clone());

        let mut vehicle = Vehicle {
            vin: vin.as_str().to_string(),
            wmi: wmi.clone(),
            make,
            ..Default::default()
        };

        if let Some(row) = make_row.take() {
            if !row.country.is_empty() {
                vehicle.plant_country = Some(row.country);
            }
            if !row.region.is_empty() {
                vehicle.region = Some(row.region);
            }
        }

        if let Ok(y) = year::decode(&vin, current_year()) {
            vehicle.model_year = Some(y);
        }
        if vehicle.region.is_none() {
            if let Some(region) = crate::wmi::region(vin.as_str().chars().next().unwrap_or('\0')) {
                vehicle.region = Some(region.to_string());
            }
        }
        self.fill_pattern_attrs(&vin, &mut vehicle);
        vehicle
    }

    fn fill_pattern_attrs(&self, vin: &Vin, vehicle: &mut Vehicle) {
        let Some(schemas) = self.wmi_schema.get(vin.wmi()) else {
            return;
        };
        let mut groups: HashMap<Element, Vec<(LookupRow, f64)>> = HashMap::new();
        for sch in &schemas {
            let Some(lookups) = self.schema_lookup.get(&sch.id) else {
                continue;
            };
            for lk in lookups {
                let Some(elem) = Element::from_str(&lk.element) else {
                    continue;
                };
                let conf = pattern::confidence(&lk.pattern, vin.vds(), vin.vis());
                if conf > elem.confidence_cutoff() {
                    groups.entry(elem).or_default().push((lk, conf));
                }
            }
        }
        for (elem, mut group) in groups {
            group.sort_by(|a, b| {
                b.0.weight
                    .cmp(&a.0.weight)
                    .then(b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
            });
            if let Some((top, _)) = group.into_iter().next() {
                elem.apply(vehicle, top.value);
            }
        }
    }
}

fn current_year() -> u32 {
    use time::OffsetDateTime;
    OffsetDateTime::now_utc().year() as u32
}
