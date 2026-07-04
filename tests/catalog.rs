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

/// Make canonicalization: zero-model duplicate makes collapse onto their
/// populated canonical sibling (via the shared MAKE_ALIASES table + merge_makes
/// twin-drop), and the empty twins are gone from the make index.
#[test]
fn make_canon_collapses_zero_model_twins() {
    let cat = Catalog::new().expect("open shipped catalog");

    // duplicate make -> populated canon
    assert_eq!(cat.resolve_make("DS").as_deref(), Some("DS AUTOMOBILES"));
    assert_eq!(
        cat.resolve_make("DS 7 CROSSBACK").as_deref(),
        Some("DS AUTOMOBILES")
    );
    assert_eq!(
        cat.resolve_make("MERCEDES-AMG").as_deref(),
        Some("MERCEDES-BENZ")
    );
    assert_eq!(
        cat.resolve_make("ROLLS ROYCE").as_deref(),
        Some("ROLLS-ROYCE")
    );
    assert_eq!(
        cat.resolve_make("ROLLS-ROYCE").as_deref(),
        Some("ROLLS-ROYCE")
    );

    // the empty twins no longer live in makes.fst (so resolve_make can't
    // short-circuit onto them)
    assert!(!cat.has_make("DS"), "empty 'DS' twin still in makes.fst");
    assert!(!cat.has_make("MERCEDES-AMG"));
    assert!(!cat.has_make("ROLLS ROYCE"));
    // the canonical survivors are present
    assert!(cat.has_make("DS AUTOMOBILES"));
    assert!(cat.has_make("MERCEDES-BENZ"));
    assert!(cat.has_make("ROLLS-ROYCE"));

    // the canon actually carries models now
    assert!(
        !cat.models_for_make("DS AUTOMOBILES").is_empty(),
        "DS AUTOMOBILES ships zero models"
    );
    let rr = cat.models_for_make("ROLLS-ROYCE");
    assert!(rr.iter().any(|m| m == "CULLINAN"), "ROLLS-ROYCE: {rr:?}");
    assert!(rr.iter().any(|m| m == "PHANTOM"));
}

/// Model top-ups: one spot-check per make/section in the supplement layer.
#[test]
fn model_topups_present() {
    let cat = Catalog::new().expect("open shipped catalog");
    let has = |mk: &str, model: &str| {
        let ms = cat.models_for_make(mk);
        assert!(
            ms.iter().any(|m| m == model),
            "{mk} missing {model}: {ms:?}"
        );
    };

    has("IVECO", "DAILY");
    has("RENAULT", "MASTER");
    has("RENAULT", "TRAFIC");
    has("CITROEN", "JUMPER");
    has("CITROEN", "SPACETOURER");
    has("FORD", "TOURNEO CUSTOM");
    has("FORD", "TRANSIT CUSTOM");
    has("VOLKSWAGEN", "CARAVELLE");
    has("VOLKSWAGEN", "TAYRON");
    has("FIAT", "TALENTO");
    has("NISSAN", "INTERSTAR");
    // standalone IX35 alongside the pre-existing "IX35 TUCSON"
    has("HYUNDAI", "IX35");
    has("HYUNDAI", "STARIA");
    has("DACIA", "BIGSTER");
    has("SKODA", "ELROQ");
    has("NIO", "EL6");
    has("ZEEKR", "8X");
    has("JETOUR", "G700");
    has("FANGCHENGBAO", "TI7");
    has("MG", "EHS");
    has("GEELY", "GALAXY");
    has("BYD", "SEALION 07");
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
