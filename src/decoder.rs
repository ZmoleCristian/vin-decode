use std::collections::HashMap;
use std::path::Path;

use crate::data::{LookupRow, MakeRow, SchemaRow, VinRuleRow};
use crate::element::Element;
use crate::maps::{FstMap, data_dir};
use crate::types::{Vehicle, Vin};
use crate::{Error, check_digit, pattern};

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
    wmi_rules: Option<FstMap<VinRuleRow>>,
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

    /// Open the decoder against an explicit data directory. The `wmi_rules`
    /// curated table is optional — only loaded if `wmi_rules.fst` exists.
    pub fn open(dir: &Path) -> crate::Result<Self> {
        let wmi_rules = if dir.join("wmi_rules.fst").exists() {
            Some(FstMap::open(dir)?)
        } else {
            None
        };
        Ok(Decoder {
            wmi_make: FstMap::open(dir)?,
            wmi_schema: FstMap::open(dir)?,
            schema_lookup: FstMap::open(dir)?,
            wmi_rules,
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
        let mut make_row = self.wmi_make.get(&wmi).and_then(|mut rows| rows.pop());

        let mut vehicle = Vehicle {
            vin: vin.as_str().to_string(),
            wmi: wmi.clone(),
            make: make_row.as_ref().map(|r| ascii_fold(&r.name)),
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
        if vehicle.region.is_none() {
            if let Some(region) = crate::wmi::region(vin.as_str().chars().next().unwrap_or('\0')) {
                vehicle.region = Some(region.to_string());
            }
        }

        // Curated VIN rules first — they can override make (e.g. UU1 → DACIA
        // on HSD prefix vs RENAULT default) and supply model when vPIC has no
        // pattern coverage. Pattern decode runs after and can refine model.
        self.apply_wmi_rules(&vin, &mut vehicle);

        // model_year is intentionally never filled here. SAE-J853 year codes
        // map to TWO candidate years 30y apart and brands disagree on which
        // VIN position carries the year. Returning a guessed year was wrong
        // more often than helpful on real corpora — consumers who want raw
        // candidates can call `Vin::year_candidates()` and decide themselves.

        self.fill_pattern_attrs(&vin, &mut vehicle);

        // plant_country: the VIN's ISO 3779 country code (positions 1-2) is the
        // authoritative source. The WMI metadata table mixes scraped sources
        // (some plain wrong — e.g. a Sunderland Nissan tagged Switzerland) and
        // vPIC pattern data is US-centric and FK-leaky, so the country range
        // encoded in the VIN itself wins whenever it maps. Unmapped prefixes
        // fall back to whatever the table/pattern supplied (already guarded
        // against unresolved numeric FKs).
        if let Some(country) = crate::country::country_from_code(vin.country_code()) {
            vehicle.plant_country = Some(country.to_string());
        }

        vehicle
    }

    /// Apply the longest-matching curated `wmi_rules` row for this VIN.
    /// Non-empty `make`/`model` fields overwrite whatever was previously set;
    /// empty fields are left alone.
    fn apply_wmi_rules(&self, vin: &Vin, vehicle: &mut Vehicle) {
        let Some(wmi_rules) = &self.wmi_rules else {
            return;
        };
        let Some(rules) = wmi_rules.get(vin.wmi()) else {
            return;
        };
        let after_wmi = &vin.as_str()[3..];
        for rule in rules {
            if !after_wmi.starts_with(&rule.remainder) {
                continue;
            }
            if !rule.make.is_empty() {
                vehicle.make = Some(rule.make.clone());
            }
            if !rule.model.is_empty() {
                vehicle.model = Some(rule.model.clone());
            }
            return;
        }
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
            group.sort_by(rank_candidates);
            if let Some((top, _)) = group.into_iter().next() {
                elem.apply(vehicle, top.value);
            }
        }
    }
}

/// Order two competing pattern rows for the same element, best first.
///
/// Primary keys are the vPIC element weight then the pattern-match confidence
/// — the real signal. But some vPIC snapshots ship no weight column (every
/// `weight` is 0) and equally specific patterns tie on confidence too, so the
/// top two keys collapse and the winner would otherwise fall to data row order
/// — which the SQL dump does not pin down, making decode nondeterministic.
///
/// Two further keys keep it deterministic AND correct: a resolved human name
/// always beats a still-numeric foreign-key id (`"Model 3"` over `"29000005"`),
/// and the value string itself breaks any final tie.
fn rank_candidates(a: &(LookupRow, f64), b: &(LookupRow, f64)) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    b.0.weight
        .cmp(&a.0.weight)
        .then(b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal))
        .then_with(|| looks_like_fk(&a.0.value).cmp(&looks_like_fk(&b.0.value)))
        .then_with(|| a.0.value.cmp(&b.0.value))
}

/// A value that is entirely ASCII digits is an unresolved foreign-key id rather
/// than a human-readable attribute — rank it below any real name.
fn looks_like_fk(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit())
}

/// Canonicalise a make string for catalog lookups: uppercase + collapse
/// hyphens to spaces + ASCII-fold diacritics. Aligns `wmi_make` rows
/// (`"MERCEDES-BENZ"`, `"CITROËN"`) with `eu_brand_models` rows
/// (`"MERCEDES BENZ"`, `"CITROEN"`).
pub(crate) fn normalize_make(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
        } else {
            for u in ch.to_uppercase() {
                out.push(ascii_fold_char(u));
            }
        }
    }
    out
}

/// Strip diacritics from a string by ASCII-folding common Latin accents.
/// Used to clean upstream sources that ship `CITROËN`, `ŠKODA`, `BJØRN`, etc.
/// Non-foldable characters are passed through unchanged.
pub(crate) fn ascii_fold(s: &str) -> String {
    s.chars().map(ascii_fold_char).collect()
}

fn ascii_fold_char(c: char) -> char {
    match c {
        'À'..='Å' | 'à'..='å' => {
            if c.is_ascii_uppercase() || c.is_uppercase() {
                'A'
            } else {
                'a'
            }
        }
        'Ç' => 'C',
        'ç' => 'c',
        'È'..='Ë' => 'E',
        'è'..='ë' => 'e',
        'Ì'..='Ï' => 'I',
        'ì'..='ï' => 'i',
        'Ñ' => 'N',
        'ñ' => 'n',
        'Ò'..='Ö' | 'Ø' => 'O',
        'ò'..='ö' | 'ø' => 'o',
        'Ù'..='Ü' => 'U',
        'ù'..='ü' => 'u',
        'Ý' | 'Ÿ' => 'Y',
        'ý' | 'ÿ' => 'y',
        'Š' => 'S',
        'š' => 's',
        'Ž' => 'Z',
        'ž' => 'z',
        _ => c,
    }
}

#[cfg(test)]
mod rank_tests {
    use super::{looks_like_fk, rank_candidates};
    use crate::data::LookupRow;

    fn cand(value: &str, weight: u32, conf: f64) -> (LookupRow, f64) {
        (
            LookupRow {
                pattern: "*".into(),
                element: "Model".into(),
                value: value.into(),
                weight,
            },
            conf,
        )
    }

    #[test]
    fn fk_detection() {
        assert!(looks_like_fk("29000005"));
        assert!(!looks_like_fk("Model 3"));
        assert!(!looks_like_fk("")); // empty is not a numeric FK
        assert!(!looks_like_fk("A4"));
    }

    /// The regression that took the cron down: equal weight (0) + equal
    /// confidence, resolved name must beat the raw FK regardless of input order.
    #[test]
    fn resolved_name_beats_raw_fk_either_order() {
        for mut g in [
            [cand("29000005", 0, 1.0), cand("Model 3", 0, 1.0)],
            [cand("Model 3", 0, 1.0), cand("29000005", 0, 1.0)],
        ] {
            g.sort_by(rank_candidates);
            assert_eq!(g[0].0.value, "Model 3");
        }
    }

    #[test]
    fn weight_then_confidence_dominate() {
        let mut g = [cand("Model 3", 0, 1.0), cand("29000005", 5, 0.1)];
        g.sort_by(rank_candidates);
        assert_eq!(g[0].0.value, "29000005", "a weighted row wins outright");

        let mut g = [cand("low", 0, 0.2), cand("high", 0, 0.9)];
        g.sort_by(rank_candidates);
        assert_eq!(g[0].0.value, "high", "higher confidence breaks weight tie");
    }

    #[test]
    fn fully_tied_is_order_independent() {
        let mut a = [cand("Zeta", 0, 1.0), cand("Alpha", 0, 1.0)];
        let mut b = [cand("Alpha", 0, 1.0), cand("Zeta", 0, 1.0)];
        a.sort_by(rank_candidates);
        b.sort_by(rank_candidates);
        assert_eq!(a[0].0.value, "Alpha");
        assert_eq!(a[0].0.value, b[0].0.value);
    }
}
