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
