mod common;

use proptest::prelude::*;
use tempfile::TempDir;
use vin_decode::data::{LookupRow, MakeRow, SchemaRow};
use vin_decode::{Catalog, Decoder, Vin};

const VIN_CHARS: &str = "0123456789ABCDEFGHJKLMNPRSTUVWXYZ";

fn vin_char_strategy() -> impl Strategy<Value = char> {
    (0..VIN_CHARS.len()).prop_map(|i| VIN_CHARS.as_bytes()[i] as char)
}

fn raw_vin_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(vin_char_strategy(), 17).prop_map(|chars| chars.iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn random_alnum_vins_parse_or_reject_cleanly(raw in raw_vin_strategy()) {
        if let Ok(v) = Vin::new(&raw) {
            prop_assert_eq!(v.as_str().len(), 17);
            prop_assert_eq!(v.wmi().len(), 3);
            prop_assert_eq!(v.vds().len(), 6);
            prop_assert_eq!(v.vis().len(), 8);
        }
    }

    #[test]
    fn lowercased_vins_parse_to_canonical_uppercase(raw in raw_vin_strategy()) {
        let lowered = raw.to_ascii_lowercase();
        if let Ok(v) = Vin::new(&lowered) {
            prop_assert_eq!(v.as_str(), raw.to_ascii_uppercase());
        }
    }
}

#[test]
fn build_with_synthetic_500_makes_works() {
    let mut wmis: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for i in 0..500 {
        wmis.insert(synthetic_wmi(i));
    }
    let wmis: Vec<String> = wmis.into_iter().collect();

    let mut wmi_make: Vec<(String, Vec<MakeRow>)> = Vec::new();
    let mut wmi_schema: Vec<(String, Vec<SchemaRow>)> = Vec::new();
    let mut schema_lookup_unsorted: Vec<(String, Vec<LookupRow>)> = Vec::new();

    for (i, wmi) in wmis.iter().enumerate() {
        wmi_make.push((
            wmi.clone(),
            vec![MakeRow {
                name: format!("Make{i:04}"),
                country: String::new(),
                region: String::new(),
            }],
        ));
        let sid = format!("S{i:05}");
        wmi_schema.push((wmi.clone(), vec![SchemaRow { id: sid.clone() }]));
        schema_lookup_unsorted.push((
            sid,
            vec![LookupRow {
                pattern: "A****".to_string(),
                element: "Model".to_string(),
                value: format!("Model{i}"),
                weight: 99,
            }],
        ));
    }
    let mut schema_lookup = schema_lookup_unsorted;
    schema_lookup.sort_by(|a, b| a.0.cmp(&b.0));

    let dir = TempDir::new().unwrap();
    vin_decode::build::build_all(&wmi_make, &wmi_schema, &schema_lookup, dir.path()).unwrap();

    let cat = Catalog::open(dir.path()).unwrap();
    assert_eq!(cat.make_count(), wmis.len() as u64);
    assert!(cat.has_make("MAKE0042"));

    let dec = Decoder::open(dir.path()).unwrap();
    let test_wmi = &wmis[10];
    let raw = pad_to_vin(test_wmi);
    let v = dec.decode_unchecked(&raw).unwrap();
    assert_eq!(v.wmi, *test_wmi);
}

#[test]
fn fixture_smoke_works() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    assert!(dec.decode("1HGCM82633A004352").is_ok());
}

#[test]
fn batch_decode_returns_input_order() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let vins = [
        "1HGCM82633A004352",
        "2FTEF14H8TCA73155",
        "5YJ3E1EA0KF000316",
    ];
    let out = dec.decode_batch(&vins);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].as_ref().unwrap().make.as_deref(), Some("Honda"));
    assert_eq!(out[1].as_ref().unwrap().make.as_deref(), Some("Ford"));
    assert_eq!(out[2].as_ref().unwrap().make.as_deref(), Some("Tesla"));
}

fn synthetic_wmi(seed: usize) -> String {
    let chars: Vec<char> = VIN_CHARS.chars().collect();
    let a = chars[seed % chars.len()];
    let b = chars[(seed / chars.len()) % chars.len()];
    let c = chars[(seed / (chars.len() * chars.len())) % chars.len()];
    format!("{a}{b}{c}")
}

fn pad_to_vin(wmi: &str) -> String {
    let suffix = "AB12345C9D0001";
    let s = format!("{wmi}{suffix}");
    debug_assert_eq!(s.len(), 17);
    s
}
