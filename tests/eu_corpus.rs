//! EU corpus integration test.
//!
//! Source: a Romanian car dealership inventory of 124 real EU VINs (Audi,
//! BMW, VW, Skoda, Mercedes, Renault, Dacia, Opel, Fiat, Citroen, Ford EU,
//! Hyundai, KIA, Lexus, Land Rover, Mini, Nissan, Porsche, Seat, Toyota,
//! Volvo). Asserts per-field coverage thresholds so EU-coverage regressions
//! are caught in CI.
//!
//! Format: `vin,make,model,year` (lowercase, kebab-case for hyphenated names).

use vin_decode::{Decoder, Vin};

fn open_decoder() -> Decoder {
    Decoder::new().expect("open decoder via embedded data")
}

const CORPUS: &str = include_str!("data/eu_corpus.csv");

struct Row<'a> {
    vin: &'a str,
    make: &'a str,
    model: &'a str,
}

fn rows() -> Vec<Row<'static>> {
    CORPUS
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut parts = l.splitn(4, ',');
            let vin = parts.next().expect("vin");
            let make = parts.next().expect("make");
            let model = parts.next().expect("model");
            // 4th field (year) is intentionally ignored — decoder no longer
            // emits model_year, and dealer-recorded years are unreliable
            // ground truth in the first place.
            Row { vin, make, model }
        })
        .collect()
}

fn norm(s: &str) -> String {
    s.to_ascii_uppercase().replace('-', " ")
}

#[test]
fn corpus_make_coverage() {
    let dec = open_decoder();
    let total = rows().len();
    let mut hits = 0;
    let mut misses = Vec::new();
    for r in rows() {
        let v = match dec.decode_unchecked(r.vin) {
            Ok(v) => v,
            Err(e) => {
                misses.push(format!("[{}] decode_unchecked failed: {e}", r.vin));
                continue;
            }
        };
        let want = norm(r.make);
        let got = v.make.as_deref().map(norm).unwrap_or_default();
        let ok = got.contains(&want) || want.contains(&got) && !got.is_empty();
        if ok {
            hits += 1;
        } else {
            misses.push(format!(
                "[{}] make={:?} wanted-substring {:?}",
                r.vin, v.make, r.make
            ));
        }
    }
    let pct = hits * 100 / total.max(1);
    if !misses.is_empty() {
        for m in &misses {
            eprintln!("{m}");
        }
    }
    assert!(
        pct >= 95,
        "[eu_corpus make] {hits}/{total} ({pct}%); expected ≥95%"
    );
}

#[test]
fn corpus_model_year_always_none() {
    // The decoder no longer guesses model year — too unreliable across EU
    // brands. Asserting None here is a regression guard: if someone wires
    // year decoding back into Decoder::decode_inner, this test fires and
    // forces a deliberate decision.
    let dec = open_decoder();
    for r in rows() {
        let Ok(v) = dec.decode_unchecked(r.vin) else {
            continue;
        };
        assert!(
            v.model_year.is_none(),
            "[{}] model_year={:?} — decoder should not auto-pick year",
            r.vin,
            v.model_year
        );
    }
}

#[test]
fn corpus_year_candidates_available_when_decodable() {
    // Sanity: the raw `year_candidates()` API still works for VINs whose
    // pos-10 carries a valid SAE-J853 code. Consumers can use this to make
    // their own year decision when they know which cycle to pick.
    let mut decodable = 0;
    for r in rows() {
        let Ok(parsed) = Vin::new(r.vin) else {
            continue;
        };
        if !parsed.year_candidates().is_empty() {
            decodable += 1;
        }
    }
    let total = rows().len();
    let pct = decodable * 100 / total.max(1);
    assert!(
        pct >= 70,
        "[eu_corpus year_candidates] {decodable}/{total} ({pct}%) had decodable codes; expected ≥70%"
    );
}

#[test]
fn corpus_model_coverage() {
    let dec = open_decoder();
    let total = rows().len();
    let mut hits = 0;
    let mut misses = Vec::new();
    for r in rows() {
        let Ok(v) = dec.decode_unchecked(r.vin) else {
            continue;
        };
        let Some(got_raw) = v.model.as_deref() else {
            misses.push(format!("[{}] model=None wanted {}", r.vin, r.model));
            continue;
        };
        let got = norm(got_raw);
        let want = norm(r.model);
        // hits if either contains the other (e.g., catalog "1 SERIES" vs Romanian "seria-1" → "SERIA 1" vs "1 SERIES")
        // ignore numeric/series-of-series fuzziness via core token contains
        let core_want = want.split_whitespace().next().unwrap_or("");
        let ok = got.contains(&want) || got.contains(core_want) && !core_want.is_empty();
        if ok {
            hits += 1;
        } else {
            misses.push(format!(
                "[{}] model={:?} wanted {:?}",
                r.vin, got_raw, r.model
            ));
        }
    }
    let pct = hits * 100 / total.max(1);
    if !misses.is_empty() {
        for m in misses.iter().take(15) {
            eprintln!("{m}");
        }
    }
    assert!(
        pct >= 60,
        "[eu_corpus model] {hits}/{total} ({pct}%); expected ≥60%"
    );
}

#[test]
fn corpus_plant_country_coverage() {
    let dec = open_decoder();
    let total = rows().len();
    let mut hits = 0;
    for r in rows() {
        let Ok(v) = dec.decode_unchecked(r.vin) else {
            continue;
        };
        if v.plant_country.is_some() {
            hits += 1;
        }
    }
    let pct = hits * 100 / total.max(1);
    assert!(
        pct >= 95,
        "[eu_corpus plant_country] {hits}/{total} ({pct}%); expected ≥95%"
    );
}

#[test]
fn corpus_models_are_not_long_fk_ids() {
    // 4+-digit pure-numeric model names are vPIC FK leakage (real models like
    // "500" / "911" / "850" are 3-digit max). 1-3 digit numerics are legitimate.
    let dec = open_decoder();
    for r in rows() {
        let Ok(v) = dec.decode_unchecked(r.vin) else {
            continue;
        };
        if let Some(model) = &v.model {
            let is_long_numeric =
                model.chars().all(|c| c.is_ascii_digit()) && model.len() >= 4 && model.len() < 6;
            assert!(
                !is_long_numeric,
                "[{}] model={model:?} looks like an unresolved FK",
                r.vin
            );
        }
    }
}
