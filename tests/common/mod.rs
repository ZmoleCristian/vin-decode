use std::path::Path;

use vin_decode::data::{LookupRow, MakeRow, ModelRow, SchemaRow};

pub fn fixture_make_rows() -> Vec<(String, Vec<MakeRow>)> {
    vec![
        (
            "1HG".to_string(),
            vec![MakeRow {
                name: "Honda".to_string(),
            }],
        ),
        (
            "2FT".to_string(),
            vec![MakeRow {
                name: "Ford".to_string(),
            }],
        ),
        (
            "5YJ".to_string(),
            vec![MakeRow {
                name: "Tesla".to_string(),
            }],
        ),
    ]
}

pub fn fixture_schema_rows() -> Vec<(String, Vec<SchemaRow>)> {
    vec![
        (
            "1HG".to_string(),
            vec![SchemaRow {
                id: "100".to_string(),
            }],
        ),
        (
            "2FT".to_string(),
            vec![SchemaRow {
                id: "200".to_string(),
            }],
        ),
        (
            "5YJ".to_string(),
            vec![SchemaRow {
                id: "300".to_string(),
            }],
        ),
    ]
}

pub fn fixture_lookup_rows() -> Vec<(String, Vec<LookupRow>)> {
    vec![
        (
            "100".to_string(),
            vec![
                LookupRow {
                    pattern: "CM82*".to_string(),
                    element: "Model".to_string(),
                    value: "Civic".to_string(),
                    weight: 99,
                },
                LookupRow {
                    pattern: "C****".to_string(),
                    element: "BodyClass".to_string(),
                    value: "Sedan".to_string(),
                    weight: 10,
                },
                LookupRow {
                    pattern: "C****".to_string(),
                    element: "FuelTypePrimary".to_string(),
                    value: "Gasoline".to_string(),
                    weight: 10,
                },
                LookupRow {
                    pattern: "C****".to_string(),
                    element: "Doors".to_string(),
                    value: "4".to_string(),
                    weight: 10,
                },
            ],
        ),
        (
            "200".to_string(),
            vec![
                LookupRow {
                    pattern: "EF14*".to_string(),
                    element: "Model".to_string(),
                    value: "F-150".to_string(),
                    weight: 99,
                },
                LookupRow {
                    pattern: "E****".to_string(),
                    element: "BodyClass".to_string(),
                    value: "Pickup".to_string(),
                    weight: 10,
                },
                LookupRow {
                    pattern: "E****".to_string(),
                    element: "FuelTypePrimary".to_string(),
                    value: "Gasoline".to_string(),
                    weight: 10,
                },
            ],
        ),
        (
            "300".to_string(),
            vec![
                LookupRow {
                    pattern: "3E1EA".to_string(),
                    element: "Model".to_string(),
                    value: "Model 3".to_string(),
                    weight: 99,
                },
                LookupRow {
                    pattern: "3****".to_string(),
                    element: "BodyClass".to_string(),
                    value: "Sedan".to_string(),
                    weight: 10,
                },
                LookupRow {
                    pattern: "3****".to_string(),
                    element: "FuelTypePrimary".to_string(),
                    value: "Electric".to_string(),
                    weight: 99,
                },
            ],
        ),
    ]
}

pub fn build_fixture(out_dir: &Path) {
    let make = fixture_make_rows();
    let schema = fixture_schema_rows();
    let lookup = fixture_lookup_rows();
    vin_decode::build::build_all(&make, &schema, &lookup, out_dir).expect("build_all");
}

#[allow(dead_code)]
pub fn assert_models_contains(catalog: &vin_decode::Catalog, make: &str, expected: &[&str]) {
    let got = catalog.models_for_make(make);
    for e in expected {
        assert!(
            got.iter().any(|g| g == e),
            "expected `{}` in models for `{}`, got {:?}",
            e,
            make,
            got
        );
    }
}

#[allow(dead_code)]
pub fn _unused_silencer(_: ModelRow) {}
