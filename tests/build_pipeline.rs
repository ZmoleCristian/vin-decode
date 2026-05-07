mod common;

use tempfile::TempDir;
use vin_decode::data::{LookupRow, MakeRow};
use vin_decode::{FstMap, FstSet};

#[test]
fn build_writes_all_expected_files() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    for f in [
        "wmi_make.fst",
        "wmi_make.bin",
        "wmi_schema.fst",
        "wmi_schema.bin",
        "schema_lookup.fst",
        "schema_lookup.bin",
        "make_models.fst",
        "make_models.bin",
        "makes.fst",
    ] {
        let p = dir.path().join(f);
        assert!(p.exists(), "missing {}", f);
        let len = std::fs::metadata(&p).unwrap().len();
        assert!(len > 0, "{} is empty", f);
    }
}

#[test]
fn make_fst_lookup_roundtrip() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let map: FstMap<MakeRow> = FstMap::open(dir.path()).unwrap();
    let rows = map.get("1HG").expect("1HG present");
    assert_eq!(rows[0].name, "Honda");
    let rows = map.get("5YJ").expect("5YJ present");
    assert_eq!(rows[0].name, "Tesla");
    assert!(map.get("ZZZ").is_none());
}

#[test]
fn lookup_fst_returns_multiple_rows() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let map: FstMap<LookupRow> = FstMap::open(dir.path()).unwrap();
    let rows = map.get("100").expect("schema 100");
    assert_eq!(rows.len(), 4);
    assert!(
        rows.iter()
            .any(|r| r.element == "Model" && r.value == "Civic")
    );
    assert!(
        rows.iter()
            .any(|r| r.element == "BodyClass" && r.value == "Sedan")
    );
}

#[test]
fn makes_set_contains_uppercase_keys() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let set = FstSet::open(&dir.path().join("makes.fst")).unwrap();
    assert!(set.contains("HONDA"));
    assert!(set.contains("FORD"));
    assert!(set.contains("TESLA"));
    assert!(!set.contains("Honda"));
    assert_eq!(set.len(), 3);
}

#[test]
fn make_models_derived_correctly() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let map: FstMap<vin_decode::data::ModelRow> = FstMap::open(dir.path()).unwrap();
    let honda_models = map.get("HONDA").expect("HONDA models");
    assert!(honda_models.iter().any(|m| m.name == "Civic"));
    let ford_models = map.get("FORD").expect("FORD models");
    assert!(ford_models.iter().any(|m| m.name == "F-150"));
    let tesla_models = map.get("TESLA").expect("TESLA models");
    assert!(tesla_models.iter().any(|m| m.name == "Model 3"));
}

#[test]
fn build_from_csv_parses_files() {
    let dir = TempDir::new().unwrap();
    let csv_dir = dir.path().join("csv");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&csv_dir).unwrap();
    std::fs::write(
        csv_dir.join("wmi_make.csv"),
        "Wmi\tMake\n1HG\tHonda\n2FT\tFord\n5YJ\tTesla\n",
    )
    .unwrap();
    std::fs::write(
        csv_dir.join("wmi_schema_id.csv"),
        "Wmi\tschema_id\n1HG\t100\n2FT\t200\n5YJ\t300\n",
    )
    .unwrap();
    std::fs::write(
        csv_dir.join("schema_id_lookup.csv"),
        "schema_id\tPattern\tElementCode\tAttributeId\tElementWeight\n\
         100\tCM82*\tModel\tCivic\t99\n\
         100\tC****\tBodyClass\tSedan\t10\n\
         200\tEF14*\tModel\tF-150\t99\n\
         300\t3E1EA\tModel\tModel 3\t99\n",
    )
    .unwrap();
    vin_decode::build::build_from_csv(&csv_dir, &out_dir).unwrap();
    let map: FstMap<MakeRow> = FstMap::open(&out_dir).unwrap();
    assert_eq!(map.get("1HG").unwrap()[0].name, "Honda");
}

#[test]
fn empty_input_produces_empty_maps() {
    let dir = TempDir::new().unwrap();
    vin_decode::build::build_all(&[], &[], &[], dir.path()).unwrap();
    let map: FstMap<MakeRow> = FstMap::open(dir.path()).unwrap();
    assert!(map.is_empty());
    let set = FstSet::open(&dir.path().join("makes.fst")).unwrap();
    assert!(set.is_empty());
}
