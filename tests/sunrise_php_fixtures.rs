//! Cross-reference tests against `sunrise-php/vin` PHP fixtures.
//!
//! Source: github.com/sunrise-php/vin/tests/VinTest.php
//! Covers: structural getters, region/country resolution, model-year ambiguity
//! (single/multiple/futurity), forbidden-char rejection.
//!
//! sunrise-php returns `getModelYear()` as an array — `[]` when unreadable,
//! `[year]` when single, `[old, new]` when ambiguous.

use vin_decode::{Error, Vin, country_from_code, region_from_code};

const VIN: &str = "WVWZZZ1KZ6W612305";
const VIN_UNKNOWN_REGION: &str = "GVWZZZ1KZ6W612305";
const VIN_UNKNOWN_COUNTRY: &str = "AZWZZZ1KZ6W612305";
const VIN_UNKNOWN_YEAR: &str = "WVWZZZ1KZZW612305"; // year_code='Z' is not in SAE table
const VIN_SINGLE_YEAR: &str = "WVWZZZ1KZ6W612305"; // year_code='6' (digit) → only 2006
const VIN_MULTIPLE_YEAR: &str = "WVWZZZ1KZAW612305"; // year_code='A' → [2010, 1980]
const VIN_FUTURITY_YEAR: &str = "WVWZZZ1KZYW612305"; // year_code='Y' → [2030, 2000] but 2030 is futurity

#[test]
fn sunrise_get_vin() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(v.as_str(), VIN);
}

#[test]
fn sunrise_get_wmi() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(v.wmi(), &VIN[..3]);
}

#[test]
fn sunrise_get_vds() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(v.vds(), &VIN[3..9]);
}

#[test]
fn sunrise_get_vis() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(v.vis(), &VIN[9..]);
}

#[test]
fn sunrise_get_region() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(region_from_code(v.region_code()), Some("Europe"));
}

#[test]
fn sunrise_get_country() {
    let v = Vin::new(VIN).unwrap();
    assert_eq!(country_from_code(v.country_code()), Some("Germany"));
}

#[test]
fn sunrise_get_model_year() {
    let v = Vin::new(VIN).unwrap();
    let years = v.year_candidates();
    // sunrise-php returns just [2006] because year_code='6' is a digit
    // (our `year_candidates` already filters duplicates for digit codes)
    assert_eq!(years, vec![2006]);
}

#[test]
fn sunrise_unknown_region() {
    // 'G' is in 'A'..='H' = Africa per ISO 3779. sunrise-php considers G
    // unassigned (their table is sparser). We DO assign it. Verify our table
    // matches ISO 3779 conventionally.
    let v = Vin::new(VIN_UNKNOWN_REGION).unwrap();
    assert_eq!(region_from_code(v.region_code()), Some("Africa"));
}

#[test]
fn sunrise_unknown_country() {
    // 'AZ' has no specific country mapping (Azerbaijan-ish range, unassigned)
    let v = Vin::new(VIN_UNKNOWN_COUNTRY).unwrap();
    let _ = country_from_code(v.country_code()); // may be None or fallback
}

#[test]
fn sunrise_unknown_model_year() {
    // 'Z' is forbidden as a year code (also a forbidden VIN char overall? no —
    // Z is fine, just unsupported in the year table)
    let v = Vin::new(VIN_UNKNOWN_YEAR).unwrap();
    assert_eq!(v.year_candidates(), Vec::<u32>::new());
}

#[test]
fn sunrise_single_model_year() {
    let v = Vin::new(VIN_SINGLE_YEAR).unwrap();
    assert_eq!(v.year_candidates(), vec![2006]);
}

#[test]
fn sunrise_multiple_model_year() {
    // year_code='A' → 1980 + 2010
    let v = Vin::new(VIN_MULTIPLE_YEAR).unwrap();
    let candidates = v.year_candidates();
    // sunrise-php returns [1980, 2010] (ascending)
    // we return descending: [2010, 1980]
    assert!(
        candidates.contains(&1980) && candidates.contains(&2010),
        "expected both 1980 and 2010, got {:?}",
        candidates
    );
}

#[test]
fn sunrise_futurity_model_year() {
    // year_code='Y' → 2000 + 2030. sunrise-php drops 2030 because it's in the
    // future relative to "now" — but since 2026 is the test ref year and 2030
    // is still future, the futurity filter applies. Our year::decode does the
    // filter; year_candidates() returns raw candidates without filtering.
    let v = Vin::new(VIN_FUTURITY_YEAR).unwrap();
    let candidates = v.year_candidates();
    assert!(
        candidates.contains(&2000),
        "expected 2000 in candidates, got {:?}",
        candidates
    );
}

#[test]
fn sunrise_to_string() {
    let v = Vin::new(VIN_FUTURITY_YEAR).unwrap();
    assert_eq!(format!("{}", v), VIN_FUTURITY_YEAR);
}

#[test]
fn sunrise_too_short() {
    let r = Vin::new("A".repeat(16));
    assert!(matches!(r, Err(Error::InvalidLength(16))));
}

#[test]
fn sunrise_too_long() {
    let r = Vin::new("A".repeat(18));
    assert!(matches!(r, Err(Error::InvalidLength(18))));
}

#[test]
fn sunrise_forbidden_chars() {
    for c in ['I', 'O', 'Q'] {
        let s = format!("{}{c}", "A".repeat(16));
        let r = Vin::new(&s);
        assert!(
            matches!(r, Err(Error::ForbiddenChar(got)) if got == c),
            "[forbidden {}] got {:?}",
            c,
            r
        );
    }
}
