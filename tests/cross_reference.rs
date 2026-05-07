//! Cross-reference tests against fixtures pulled from upstream OSS VIN libs.
//!
//! Each test case names its origin (vin-go, vininfo, etc) so attribution is
//! traceable. We intentionally use `decode_unchecked` because most upstream
//! fixtures use placeholder digits (`0`s) that don't pass ISO 3779.

use vin_decode::{Catalog, Decoder};

fn open_decoder() -> Decoder {
    // Decoder::new prefers VIN_DECODE_DATA_DIR but falls back to the embedded
    // data set (auto-decompressed into ~/.vin-decode-cache on first run).
    Decoder::new().expect("open decoder via embedded data")
}

fn open_catalog() -> Catalog {
    Catalog::new().expect("open catalog via embedded data")
}

/// Source: vin-go/decode_brand_test.go — synthetic placeholder VINs.
#[test]
fn vin_go_brand_fixtures() {
    let dec = open_decoder();
    let cases: &[(&str, &str)] = &[
        ("WAU00000000000000", "AUDI"),
        ("WA100000000000000", "AUDI"),
        ("TMB00000000000000", "SKODA"),
        ("WP000000000000000", "PORSCHE"),
        ("WMA00000000000000", "MAN"),
    ];
    for (vin, want) in cases {
        let v = dec.decode_unchecked(vin).unwrap_or_else(|e| {
            panic!("decode_unchecked({vin}) failed: {e}");
        });
        let got = v
            .make
            .as_deref()
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        assert!(
            got.contains(want),
            "[vin-go] vin={vin} expected make to contain `{want}`, got {got:?}"
        );
    }
}

/// Source: vininfo/tests/test_opel.py — Opel/Vauxhall, German plant, year 2012.
#[test]
fn vininfo_opel_fixture() {
    let dec = open_decoder();
    let v = dec.decode_unchecked("W0LPC6DB3CC123456").unwrap();
    let make = v.make.as_deref().unwrap_or("").to_ascii_uppercase();
    assert!(
        make.contains("OPEL") || make.contains("VAUXHALL"),
        "[vininfo opel] expected OPEL or VAUXHALL in make, got {make:?}"
    );
    let region = v.region.as_deref().unwrap_or("").to_ascii_uppercase();
    assert_eq!(region, "EUROPE", "[vininfo opel] region got {region:?}");
}

/// Source: vininfo/tests/test_renault.py — Renault, France, 2005.
#[test]
fn vininfo_renault_fixture() {
    let dec = open_decoder();
    let v = dec.decode_unchecked("VF14SRAP451234567").unwrap();
    let make = v.make.as_deref().unwrap_or("").to_ascii_uppercase();
    assert!(
        make.contains("RENAULT"),
        "[vininfo renault] expected RENAULT in make, got {make:?}"
    );
}

/// Source: vininfo/tests/test_lada.py — AvtoVAZ/Lada, Russia, 2018.
#[test]
fn vininfo_lada_fixture() {
    let dec = open_decoder();
    let v = dec.decode_unchecked("XTAGFK330JY144213").unwrap();
    let make = v.make.as_deref().unwrap_or("").to_ascii_uppercase();
    assert!(
        make.contains("LADA") || make.contains("AVTOVAZ"),
        "[vininfo lada] expected LADA/AVTOVAZ in make, got {make:?}"
    );
}

/// Source: sunrise-php/vin/tests/VinTest.php — VW, German plant.
#[test]
fn sunrise_php_vw_fixture() {
    let dec = open_decoder();
    let v = dec.decode_unchecked("WVWZZZ1KZ6W612305").unwrap();
    let make = v.make.as_deref().unwrap_or("").to_ascii_uppercase();
    assert!(
        make.contains("VOLKSWAGEN") || make.contains("VW"),
        "[sunrise-php vw] expected VOLKSWAGEN in make, got {make:?}"
    );
}

/// EU brand catalog should expose Skoda, Dacia, Cupra and friends with non-trivial
/// model lists — the original gap we set out to fix.
#[test]
fn eu_brand_catalog_coverage() {
    let cat = open_catalog();
    let must_have: &[(&str, usize)] = &[
        ("SKODA", 20),
        ("DACIA", 10),
        ("CUPRA", 5),
        ("PEUGEOT", 50),
        ("CITROEN", 30),
        ("VAUXHALL", 30),
        ("RENAULT", 50),
        ("FIAT", 50),
        ("BMW", 50),
        ("AUDI", 30),
        ("MERCEDES-BENZ", 30),
        ("VOLKSWAGEN", 30),
    ];
    for (brand, min_models) in must_have {
        let models = cat.eu_models_for(brand);
        assert!(
            models.len() >= *min_models,
            "[catalog] {brand}: expected ≥{min_models} models, got {} ({:?})",
            models.len(),
            models.iter().take(5).map(|m| &m.name).collect::<Vec<_>>()
        );
    }
}

/// Engine catalog should give us specs for at least the popular EU lineup.
#[test]
fn eu_engine_catalog_coverage() {
    let cat = open_catalog();
    let cases: &[(&str, &str, usize)] = &[
        ("DACIA", "DUSTER", 5),
        ("SKODA", "OCTAVIA", 5),
        ("PEUGEOT", "208 5 DOORS", 3),
        ("RENAULT", "CLIO", 3),
        ("BMW", "M3 SEDAN G80", 1),
        ("AUDI", "A4", 3),
    ];
    for (brand, model, min_engines) in cases {
        let engines = cat.engines_for(brand, model);
        assert!(
            engines.len() >= *min_engines,
            "[engines] {brand} {model}: expected ≥{min_engines} engines, got {}",
            engines.len()
        );
        // each engine has a name; at least 70% should have power info (some
        // upstream entries are partial)
        let mut with_power = 0;
        for e in &engines {
            assert!(!e.name.is_empty(), "engine name empty: {:?}", e);
            if e.power_kw > 0 || e.power_hp > 0 {
                with_power += 1;
            }
        }
        let pct = with_power * 100 / engines.len().max(1);
        assert!(
            pct >= 70,
            "[engines] {brand} {model}: only {pct}% engines have power info ({}/{})",
            with_power,
            engines.len()
        );
    }
}

/// Make catalog must contain the canonical EU brand list (165 brands).
#[test]
fn eu_brand_set_contains_required() {
    let cat = open_catalog();
    let required = [
        "SKODA",
        "DACIA",
        "CUPRA",
        "SEAT",
        "PEUGEOT",
        "CITROEN",
        "VAUXHALL",
        "RENAULT",
        "FIAT",
        "BMW",
        "AUDI",
        "MERCEDES-BENZ",
        "VOLKSWAGEN",
        "PORSCHE",
        "VOLVO",
        "LADA",
        "POLESTAR",
        "MINI",
    ];
    for brand in required {
        assert!(
            cat.has_make(brand),
            "[brands] missing required brand: {brand}"
        );
    }
}
