//! Post-process `schema_lookup.bin` by resolving foreign-key AttributeIds to
//! human-readable names using NHTSA's `GetVehicleVariableValuesList` JSON
//! dumps.
//!
//! The vPIC `Pattern.AttributeId` column is a foreign key into per-element
//! lookup tables (Model.Id, BodyClass.Id, etc.) for FK-typed elements, and a
//! literal string for free-text elements. Our refresh-vpic workflow originally
//! pulled the FK directly without joining; this binary back-fills that data
//! locally without needing to spin up MSSQL.
//!
//! Usage: `vin-decode-resolve-fks <in_dir> <out_dir> <nhtsa_lookups_dir>`
//!
//! `<nhtsa_lookups_dir>` must contain `<Element>.json` files produced by
//! curl-ing
//! `https://vpic.nhtsa.dot.gov/api/vehicles/GetVehicleVariableValuesList/<slug>?format=json`.
//! The element name (e.g. `Model`, `BodyClass`, `FuelTypePrimary`) is the file
//! basename. JSON shape: `{"Results": [{"Id": <int>, "Name": "<str>", ...}]}`.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

use vin_decode::FstMap;
use vin_decode::data::{LookupRow, MakeRow, ModelRow, SchemaRow};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(in_dir) = args.next() else {
        eprintln!("usage: vin-decode-resolve-fks <in_dir> <out_dir> <nhtsa_lookups_dir>");
        return ExitCode::from(2);
    };
    let Some(out_dir) = args.next() else {
        eprintln!("usage: vin-decode-resolve-fks <in_dir> <out_dir> <nhtsa_lookups_dir>");
        return ExitCode::from(2);
    };
    let Some(lookups_dir) = args.next() else {
        eprintln!("usage: vin-decode-resolve-fks <in_dir> <out_dir> <nhtsa_lookups_dir>");
        return ExitCode::from(2);
    };

    let in_dir = PathBuf::from(in_dir);
    let out_dir = PathBuf::from(out_dir);
    let lookups_dir = PathBuf::from(lookups_dir);

    let resolvers = match load_resolvers(&lookups_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("loading lookup tables failed: {e}");
            return ExitCode::from(1);
        }
    };
    eprintln!("loaded {} element resolvers", resolvers.len());

    let map: FstMap<LookupRow> = match FstMap::open(&in_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("opening schema_lookup at {} failed: {e}", in_dir.display());
            return ExitCode::from(1);
        }
    };

    // Iterate every key, resolve FKs in each row group.
    let keys = map.keys();
    eprintln!("schema groups: {}", keys.len());
    let mut resolved_rows = 0usize;
    let mut total_rows = 0usize;
    let mut groups: Vec<(String, Vec<LookupRow>)> = Vec::with_capacity(keys.len());
    for key in keys {
        let Some(rows) = map.get(&key) else {
            continue;
        };
        let mut new_rows: Vec<LookupRow> = Vec::with_capacity(rows.len());
        for row in rows {
            total_rows += 1;
            let mut new_value = row.value.clone();
            if let Some(resolver) = resolvers.get(row.element.as_str()) {
                if let Ok(id) = row.value.parse::<u32>() {
                    if let Some(name) = resolver.get(&id) {
                        new_value = name.clone();
                        resolved_rows += 1;
                    }
                }
            }
            new_rows.push(LookupRow {
                pattern: row.pattern,
                element: row.element,
                value: new_value,
                weight: row.weight,
            });
        }
        groups.push((key, new_rows));
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));

    std::fs::create_dir_all(&out_dir).ok();
    if let Err(e) = vin_decode::build::write_grouped(
        &groups,
        &out_dir.join("schema_lookup.fst"),
        &out_dir.join("schema_lookup.bin"),
    ) {
        eprintln!("write_grouped failed: {e}");
        return ExitCode::from(1);
    }
    eprintln!(
        "resolved {resolved_rows}/{total_rows} rows ({:.1}%)",
        resolved_rows as f64 * 100.0 / total_rows.max(1) as f64
    );

    // Regenerate make_models from the now-resolved schema_lookup. Without this
    // the catalog's `models_for_make` would still return numeric FK strings.
    if let Err(e) = regen_make_models(&in_dir, &groups, &out_dir) {
        eprintln!("regen_make_models failed: {e}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

/// Re-derive `make_models.fst`/`.bin` from the resolved schema_lookup so that
/// `Catalog::models_for_make` returns human-readable model names.
fn regen_make_models(
    in_dir: &std::path::Path,
    schema_lookup: &[(String, Vec<LookupRow>)],
    out_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let wmi_make_map: FstMap<MakeRow> = FstMap::open(in_dir)?;
    let wmi_schema_map: FstMap<SchemaRow> = FstMap::open(in_dir)?;
    let wmi_make: Vec<(String, Vec<MakeRow>)> = wmi_make_map
        .keys()
        .into_iter()
        .filter_map(|k| wmi_make_map.get(&k).map(|v| (k, v)))
        .collect();
    let wmi_schema: Vec<(String, Vec<SchemaRow>)> = wmi_schema_map
        .keys()
        .into_iter()
        .filter_map(|k| wmi_schema_map.get(&k).map(|v| (k, v)))
        .collect();
    let make_models = vin_decode::build::derive_make_models(&wmi_make, &wmi_schema, schema_lookup);
    vin_decode::build::write_grouped(
        &make_models,
        &out_dir.join("make_models.fst"),
        &out_dir.join("make_models.bin"),
    )?;
    eprintln!(
        "regen make_models: {} make groups",
        make_models.len()
    );
    let _ = std::any::type_name::<ModelRow>();
    Ok(())
}

fn load_resolvers(dir: &std::path::Path) -> std::io::Result<HashMap<String, HashMap<u32, String>>> {
    let mut out: HashMap<String, HashMap<u32, String>> = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let element = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let f = File::open(&path)?;
        let json: serde_json::Value =
            serde_json::from_reader(f).map_err(std::io::Error::other)?;
        let mut map: HashMap<u32, String> = HashMap::new();
        if let Some(arr) = json.get("Results").and_then(|v| v.as_array()) {
            for r in arr {
                let id = r.get("Id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let name = r
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if id > 0 && !name.is_empty() {
                    map.insert(id, name);
                }
            }
        }
        if !map.is_empty() {
            out.insert(element, map);
        }
    }
    Ok(out)
}
