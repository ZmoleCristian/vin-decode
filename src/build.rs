//! Build-time pipeline for converting vPIC tabular data into FST/rkyv lookup maps.
//!
//! Enabled via the `build` feature. End users normally don't need this — the
//! crate ships pre-built maps. This module is used by the monthly CI cron job
//! that regenerates the embedded data.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use fst::{MapBuilder, SetBuilder};
use rkyv::rancor::Error as RkyvError;

use crate::Error;
pub use crate::data::{LookupRow, MakeRow, ModelRow, SchemaRow};
use crate::data::{RkyvSer, Saveable};

/// Write a typed `key → Vec<T>` map as paired `.fst` index + `.bin` rkyv blob.
///
/// `sorted_pairs` MUST be sorted by key (FST builders require lex-ordered insertion).
pub fn write_grouped<T>(
    sorted_pairs: &[(String, Vec<T>)],
    fst_path: &Path,
    values_path: &Path,
) -> crate::Result<()>
where
    T: RkyvSer,
{
    let fst_writer = BufWriter::new(File::create(fst_path)?);
    let mut values_writer = BufWriter::new(File::create(values_path)?);
    let mut builder = MapBuilder::new(fst_writer).map_err(|e| Error::MissingData(e.to_string()))?;
    let mut offset: u64 = 0;
    for (key, values) in sorted_pairs {
        let bytes =
            rkyv::to_bytes::<RkyvError>(values).map_err(|e| Error::MissingData(e.to_string()))?;
        values_writer.write_all(&bytes)?;
        let combined = (offset << 32) | (bytes.len() as u64);
        builder
            .insert(key, combined)
            .map_err(|e| Error::MissingData(e.to_string()))?;
        offset += bytes.len() as u64;
    }
    builder
        .finish()
        .map_err(|e| Error::MissingData(e.to_string()))?;
    values_writer.flush()?;
    Ok(())
}

/// Write a sorted set of keys as a value-less `.fst` (used for the makes index).
pub fn write_set(sorted_keys: &[String], fst_path: &Path) -> crate::Result<()> {
    let writer = BufWriter::new(File::create(fst_path)?);
    let mut builder = SetBuilder::new(writer).map_err(|e| Error::MissingData(e.to_string()))?;
    for key in sorted_keys {
        builder
            .insert(key)
            .map_err(|e| Error::MissingData(e.to_string()))?;
    }
    builder
        .finish()
        .map_err(|e| Error::MissingData(e.to_string()))?;
    Ok(())
}

/// Derive a make→models reverse index by joining WMI/schema/lookup tables.
///
/// Walks every WMI's schemas, collects every lookup row with `element == "Model"`,
/// and bucket-sorts the resulting model strings under their make.
pub fn derive_make_models(
    wmi_make: &[(String, Vec<MakeRow>)],
    wmi_schema: &[(String, Vec<SchemaRow>)],
    schema_lookup: &[(String, Vec<LookupRow>)],
) -> Vec<(String, Vec<ModelRow>)> {
    let make_by_wmi: BTreeMap<&str, &str> = wmi_make
        .iter()
        .filter_map(|(wmi, rows)| rows.first().map(|r| (wmi.as_str(), r.name.as_str())))
        .collect();

    let schemas_by_wmi: BTreeMap<&str, Vec<&str>> = wmi_schema
        .iter()
        .map(|(wmi, rows)| {
            (
                wmi.as_str(),
                rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            )
        })
        .collect();

    let lookups_by_schema: BTreeMap<&str, &[LookupRow]> = schema_lookup
        .iter()
        .map(|(id, rows)| (id.as_str(), rows.as_slice()))
        .collect();

    let mut acc: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (wmi, make) in &make_by_wmi {
        let Some(schemas) = schemas_by_wmi.get(wmi) else {
            continue;
        };
        for sid in schemas {
            let Some(lookups) = lookups_by_schema.get(sid) else {
                continue;
            };
            for lk in lookups.iter() {
                if lk.element == "Model" {
                    acc.entry(make.to_ascii_uppercase())
                        .or_default()
                        .insert(lk.value.clone());
                }
            }
        }
    }

    acc.into_iter()
        .map(|(make, models)| {
            (
                make,
                models.into_iter().map(|name| ModelRow { name }).collect(),
            )
        })
        .collect()
}

/// Collect the unique sorted set of uppercase make names from a wmi_make table.
pub fn collect_makes(wmi_make: &[(String, Vec<MakeRow>)]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for (_, rows) in wmi_make {
        for r in rows {
            set.insert(r.name.to_ascii_uppercase());
        }
    }
    set.into_iter().collect()
}

/// Build the full set of FST/bin files plus the makes index and make_models reverse index.
pub fn build_all(
    wmi_make: &[(String, Vec<MakeRow>)],
    wmi_schema: &[(String, Vec<SchemaRow>)],
    schema_lookup: &[(String, Vec<LookupRow>)],
    out_dir: &Path,
) -> crate::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    write_grouped(
        wmi_make,
        &out_dir.join(format!("{}.fst", MakeRow::base_name())),
        &out_dir.join(format!("{}.bin", MakeRow::base_name())),
    )?;
    write_grouped(
        wmi_schema,
        &out_dir.join(format!("{}.fst", SchemaRow::base_name())),
        &out_dir.join(format!("{}.bin", SchemaRow::base_name())),
    )?;
    write_grouped(
        schema_lookup,
        &out_dir.join(format!("{}.fst", LookupRow::base_name())),
        &out_dir.join(format!("{}.bin", LookupRow::base_name())),
    )?;

    let makes = collect_makes(wmi_make);
    write_set(&makes, &out_dir.join("makes.fst"))?;

    let make_models = derive_make_models(wmi_make, wmi_schema, schema_lookup);
    write_grouped(
        &make_models,
        &out_dir.join(format!("{}.fst", ModelRow::base_name())),
        &out_dir.join(format!("{}.bin", ModelRow::base_name())),
    )?;
    Ok(())
}

/// Stream-build a typed map directly from a CSV file grouped by its leading column.
///
/// Avoids buffering the full CSV in memory — emits FST entries as soon as the
/// leading key changes. Caller must ensure the CSV is already sorted by the key column.
pub fn write_csv_grouped<T, F>(
    csv_path: &Path,
    fst_path: &Path,
    values_path: &Path,
    mut row_fn: F,
) -> crate::Result<()>
where
    T: RkyvSer,
    F: FnMut(&csv::StringRecord) -> Option<T>,
{
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b'\t')
        .quoting(false)
        .from_path(csv_path)?;
    let fst_writer = BufWriter::new(File::create(fst_path)?);
    let mut values_writer = BufWriter::new(File::create(values_path)?);
    let mut builder = MapBuilder::new(fst_writer).map_err(|e| Error::MissingData(e.to_string()))?;

    let mut offset: u64 = 0;
    let mut current_key: Option<String> = None;
    let mut group: Vec<T> = Vec::new();

    let flush = |builder: &mut MapBuilder<BufWriter<File>>,
                 values_writer: &mut BufWriter<File>,
                 offset: &mut u64,
                 key: &str,
                 group: &Vec<T>|
     -> crate::Result<()> {
        let bytes =
            rkyv::to_bytes::<RkyvError>(group).map_err(|e| Error::MissingData(e.to_string()))?;
        values_writer.write_all(&bytes)?;
        let combined = (*offset << 32) | (bytes.len() as u64);
        builder
            .insert(key, combined)
            .map_err(|e| Error::MissingData(e.to_string()))?;
        *offset += bytes.len() as u64;
        Ok(())
    };

    for rec in reader.records() {
        let rec = rec.map_err(|e| Error::MissingData(e.to_string()))?;
        let Some(key) = rec.get(0) else { continue };
        let Some(item) = row_fn(&rec) else { continue };
        match &current_key {
            Some(k) if k == key => group.push(item),
            Some(k) => {
                flush(&mut builder, &mut values_writer, &mut offset, k, &group)?;
                group.clear();
                group.push(item);
                current_key = Some(key.to_string());
            }
            None => {
                current_key = Some(key.to_string());
                group.push(item);
            }
        }
    }
    if let Some(k) = current_key {
        flush(&mut builder, &mut values_writer, &mut offset, &k, &group)?;
    }
    builder
        .finish()
        .map_err(|e| Error::MissingData(e.to_string()))?;
    values_writer.flush()?;
    Ok(())
}

/// Build the full map suite from a directory of vPIC CSVs.
///
/// Expects `wmi_make.csv`, `wmi_schema_id.csv`, `schema_id_lookup.csv`, each
/// sorted by their first column.
pub fn build_from_csv(csv_dir: &Path, out_dir: &Path) -> crate::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    write_csv_grouped::<MakeRow, _>(
        &csv_dir.join("wmi_make.csv"),
        &out_dir.join(format!("{}.fst", MakeRow::base_name())),
        &out_dir.join(format!("{}.bin", MakeRow::base_name())),
        |rec| {
            rec.get(1).map(|s| MakeRow {
                name: s.to_string(),
            })
        },
    )?;
    write_csv_grouped::<SchemaRow, _>(
        &csv_dir.join("wmi_schema_id.csv"),
        &out_dir.join(format!("{}.fst", SchemaRow::base_name())),
        &out_dir.join(format!("{}.bin", SchemaRow::base_name())),
        |rec| rec.get(1).map(|s| SchemaRow { id: s.to_string() }),
    )?;
    write_csv_grouped::<LookupRow, _>(
        &csv_dir.join("schema_id_lookup.csv"),
        &out_dir.join(format!("{}.fst", LookupRow::base_name())),
        &out_dir.join(format!("{}.bin", LookupRow::base_name())),
        |rec| {
            let pattern = rec.get(1)?.to_string();
            let element = rec.get(2)?.to_string();
            let value = rec.get(3)?.to_string();
            let weight = rec.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(LookupRow {
                pattern,
                element,
                value,
                weight,
            })
        },
    )?;

    let wmi_make = read_csv_grouped(&csv_dir.join("wmi_make.csv"), |rec| {
        rec.get(1).map(|s| MakeRow {
            name: s.to_string(),
        })
    })?;
    let wmi_schema = read_csv_grouped(&csv_dir.join("wmi_schema_id.csv"), |rec| {
        rec.get(1).map(|s| SchemaRow { id: s.to_string() })
    })?;
    let schema_lookup = read_csv_grouped(&csv_dir.join("schema_id_lookup.csv"), |rec| {
        let pattern = rec.get(1)?.to_string();
        let element = rec.get(2)?.to_string();
        let value = rec.get(3)?.to_string();
        let weight = rec.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        Some(LookupRow {
            pattern,
            element,
            value,
            weight,
        })
    })?;

    let makes = collect_makes(&wmi_make);
    write_set(&makes, &out_dir.join("makes.fst"))?;
    let make_models = derive_make_models(&wmi_make, &wmi_schema, &schema_lookup);
    write_grouped(
        &make_models,
        &out_dir.join(format!("{}.fst", ModelRow::base_name())),
        &out_dir.join(format!("{}.bin", ModelRow::base_name())),
    )?;
    Ok(())
}

fn read_csv_grouped<T, F>(csv_path: &Path, mut row_fn: F) -> crate::Result<Vec<(String, Vec<T>)>>
where
    F: FnMut(&csv::StringRecord) -> Option<T>,
{
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b'\t')
        .quoting(false)
        .from_path(csv_path)?;
    let mut acc: BTreeMap<String, Vec<T>> = BTreeMap::new();
    for rec in reader.records() {
        let rec = rec.map_err(|e| Error::MissingData(e.to_string()))?;
        let Some(key) = rec.get(0) else { continue };
        let Some(item) = row_fn(&rec) else { continue };
        acc.entry(key.to_string()).or_default().push(item);
    }
    Ok(acc.into_iter().collect())
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Self {
        Error::MissingData(e.to_string())
    }
}
