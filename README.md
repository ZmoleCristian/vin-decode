# vin-decode

> **Auto-updating VIN decoder.** Lookup data is regenerated monthly from the
> official NHTSA vPIC dump by a CI cron job and shipped embedded in the crate.
> No network at runtime, no manual provisioning, no stale tables.

VIN parsing and decoding for Rust, backed by the full NHTSA vPIC database
compiled into [`fst`](https://crates.io/crates/fst) +
[`rkyv`](https://crates.io/crates/rkyv) memory-mapped lookup tables.

## Example

```rust
use vin_decode::Decoder;

let dec = Decoder::new()?;
let v = dec.decode("1HGCM82633A004352")?;
assert_eq!(v.make.as_deref(), Some("Honda"));
assert_eq!(v.model_year, Some(2003));
# Ok::<(), vin_decode::Error>(())
```

## Catalog (browse all known makes / models)

```rust
use vin_decode::Catalog;

let cat = Catalog::new()?;
for make in cat.all_makes() {
    println!("{make}");
}
let models = cat.models_for_make("Honda");
# Ok::<(), vin_decode::Error>(())
```

## Features

| Feature | Default | Effect |
|---|---|---|
| `embedded` | yes | Bundles compressed lookup tables; auto-installs on first decoder construction |
| `parallel` | yes | `rayon`-powered batch decoding |
| `serde` | no | `serde::{Serialize, Deserialize}` on `Vehicle`, `Vin`, `BodyClass`, `FuelType` |
| `build` | no | Exposes the FST/CSV build pipeline (used by CI) |

## Data freshness

A weekly GitHub Actions job probes
`https://vpic.nhtsa.dot.gov/downloads/vPICList_lite_YYYY_MM.bak.zip` for new
NHTSA dumps (NHTSA publishes a fresh one mid-month). **A new release is only
cut when a newer dump exists** — if the upstream version matches the bundled
one, the job exits without committing, tagging, or publishing. So a `cargo
update` only ever pulls actual data changes; no churn-publishes.

Crate patch versions track data updates; crate minor versions track API changes.

## License

0BSD
