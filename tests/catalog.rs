mod common;

use tempfile::TempDir;
use vin_decode::{BodyType, Catalog, FuelType};

#[test]
fn all_makes_returns_sorted_uppercase() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    let makes = cat.all_makes();
    assert_eq!(makes, vec!["FORD", "HONDA", "TESLA"]);
}

#[test]
fn make_count_matches_set_size() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    assert_eq!(cat.make_count(), 3);
}

#[test]
fn has_make_case_insensitive_lookup() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    assert!(cat.has_make("Honda"));
    assert!(cat.has_make("honda"));
    assert!(cat.has_make("HONDA"));
    assert!(!cat.has_make("Yugo"));
}

#[test]
fn models_for_make_returns_known_set() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    common::assert_models_contains(&cat, "Honda", &["Civic"]);
    common::assert_models_contains(&cat, "Ford", &["F-150"]);
    common::assert_models_contains(&cat, "Tesla", &["Model 3"]);
}

#[test]
fn models_for_unknown_make_returns_empty() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    assert!(cat.models_for_make("FakeBrand").is_empty());
}

#[test]
fn make_for_model_reverse_lookup() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let cat = Catalog::open(dir.path()).unwrap();
    assert_eq!(cat.make_for_model("Civic"), vec!["HONDA"]);
    assert_eq!(cat.make_for_model("civic"), vec!["HONDA"]); // case-insensitive
    assert_eq!(cat.make_for_model("F-150"), vec!["FORD"]);
    assert!(cat.make_for_model("Nonexistent").is_empty());
}

/// Round-trip the downstream consumer contract against the real shipped
/// catalog: resolve_make canonicalises a glued listing, and the residual left
/// after stripping the make token is a model under that canonical make. Also
/// asserts every curated EXTRA_MAKES brand now ships a non-empty model list
/// (the bug this fixes: they shipped zero models).
#[test]
fn extra_models_curated_cohort_roundtrip() {
    let cat = Catalog::new().expect("open shipped catalog");

    // "DFSK SERES 3" -> make DFSK (trailing tokens dropped), residual "SERES 3".
    let m = cat.resolve_make("DFSK SERES 3").expect("DFSK resolves");
    assert_eq!(m, "DFSK");
    assert!(
        cat.models_for_make(&m).iter().any(|x| x == "SERES 3"),
        "DFSK models missing 'SERES 3': {:?}",
        cat.models_for_make(&m)
    );

    // bare-tagged "Seres 3" -> make SERES, residual "3" filed under SERES.
    let s = cat.resolve_make("SERES 3").expect("SERES resolves");
    assert_eq!(s, "SERES");
    assert!(
        cat.models_for_make(&s).iter().any(|x| x == "3"),
        "SERES models missing bare '3': {:?}",
        cat.models_for_make(&s)
    );

    // normalize_make handles the hyphen/space/&-bearing keys on lookup.
    assert!(cat.models_for_make("M-HERO").iter().any(|x| x == "917"));
    assert!(cat.models_for_make("LYNK & CO").iter().any(|x| x == "01"));

    // every curated make resolves and yields a non-empty model list.
    for mk in [
        "DFSK", "SERES", "ZEEKR", "OMODA", "JAECOO", "LEAPMOTOR", "DENZA", "VOYAH", "HONGQI",
        "MAXUS", "JETOUR", "AVATR", "BAIC", "GWM", "TANK", "ORA", "FORTHING", "BESTUNE", "SKYWELL",
        "AION", "AIWAYS", "JAC", "DONGFENG", "M-HERO", "MAEXTRO", "FANGCHENGBAO", "LINKTOUR",
        "TODAY SUNSHINE", "XIAOMI", "IM", "LYNK & CO", "KGM", "AIXAM", "LIGIER", "MICROLINO",
        "TAZZARI", "XEV", "ESTRIMA", "CHATENET", "DR", "EVO", "SWM",
    ] {
        assert!(
            !cat.models_for_make(mk).is_empty(),
            "curated make {mk} still ships zero models"
        );
    }
}

#[test]
fn body_types_static_full_list() {
    let bc = Catalog::body_types();
    assert_eq!(bc.len(), 38);
    assert!(bc.contains(&BodyType::Saloon));
    assert!(bc.contains(&BodyType::PickUp));
    assert!(!bc.contains(&BodyType::Unknown));
}

#[test]
fn fuel_types_static_full_list() {
    let ft = Catalog::fuel_types();
    assert_eq!(ft.len(), 15);
    assert!(ft.contains(&FuelType::Electric));
    assert!(ft.contains(&FuelType::Hydrogen));
    assert!(ft.contains(&FuelType::Other));
}
