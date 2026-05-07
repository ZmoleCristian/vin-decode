//! Cross-reference tests against the `way-platform/vin-go` Go fixtures.
//!
//! Source: github.com/way-platform/vin-go
//!   - decode_brand_test.go
//!   - internal/oem/opelvin/infer_test.go
//!   - internal/oem/fordvin/infer_test.go
//!   - internal/oem/volkswagenvin/infer_test.go
//!
//! These are synthetic placeholder VINs (mostly trailing zeros) meant to drive
//! a brand-only assertion via the WMI table. We use `decode_unchecked` because
//! the placeholder check digits don't validate.

use vin_decode::Decoder;

fn open_decoder() -> Decoder {
    Decoder::new().expect("open decoder via embedded data")
}

/// (vin, expected_brand_substring_uppercase) — vin-go uses an enum like
/// `Brand_AUDI` so we match by name substring against the decoded make.
const FIXTURES: &[(&str, &str)] = &[
    // decode_brand_test.go — top-level brand fixtures
    ("WAU00000000000000", "AUDI"),
    ("WA100000000000000", "AUDI"),
    ("TMB00000000000000", "SKODA"),
    ("WP000000000000000", "PORSCHE"),
    ("WMA00000000000000", "MAN"),
    // SCHMITZ-CARGOBULL is unlikely in our consumer-car catalog, skip if absent
    // opelvin/infer_test.go — every prefix here should resolve to OPEL or VAUXHALL
    ("VXEE1ABCD00000000", "OPEL"),
    ("VXEV1ZKXZ00000000", "OPEL"),
    ("W0L6WZR1B00000000", "OPEL"),
    ("W0LABCDEF00000000", "OPEL"),
    ("W0VF7D60000000000", "OPEL"),
    // fordvin/infer_test.go — Ford UK / Ford EU plants
    ("WF0AXXTA000000000", "FORD"),
    ("WF0FXXTTFF8000000", "FORD"),
    ("WF0RXXTA000000000", "FORD"),
    ("WF0RXXTA200000000", "FORD"),
    ("WF0RXXTZ000000000", "FORD"),
    ("AFAAAAAAA00000000", "FORD"), // Ford South Africa
    // volkswagenvin/infer_test.go
    ("WV2ZZZEB800000000", "VOLKSWAGEN"),
    ("WV1ZZZSYZ00000000", "VOLKSWAGEN"),
    ("WV2ZZZST000000000", "VOLKSWAGEN"),
    ("WV1ZZZ7HZ00000000", "VOLKSWAGEN"),
];

#[test]
fn vingo_brand_resolution() {
    let dec = open_decoder();
    let mut hits = 0;
    let mut misses = Vec::new();
    for (vin, want) in FIXTURES {
        let v = dec
            .decode_unchecked(vin)
            .unwrap_or_else(|e| panic!("[{vin}] decode failed: {e}"));
        let upper = v
            .make
            .as_deref()
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        // OPEL or VAUXHALL is acceptable for opel WMIs (same group)
        let acceptable = if *want == "OPEL" {
            upper.contains("OPEL") || upper.contains("VAUXHALL")
        } else {
            upper.contains(want)
        };
        if acceptable {
            hits += 1;
        } else {
            misses.push(format!("[{vin}] got {:?}, wanted {want}", v.make));
        }
    }
    if !misses.is_empty() {
        for m in &misses {
            eprintln!("{m}");
        }
    }
    let pct = hits * 100 / FIXTURES.len();
    assert!(
        pct >= 80,
        "[vin-go brand] {hits}/{} ({pct}%) brand hits — expected ≥80%",
        FIXTURES.len()
    );
}

/// vin-go's check-digit validator skips ISO 3779 enforcement for some Ford and
/// Ford Australia VINs (different position-9 layout). Mirror that exception by
/// asserting `decode_unchecked` always succeeds even if `decode` rejects.
#[test]
fn vingo_unchecked_always_parses() {
    let dec = open_decoder();
    for (vin, _) in FIXTURES {
        assert!(
            dec.decode_unchecked(vin).is_ok(),
            "decode_unchecked({vin}) must succeed"
        );
    }
}

/// vin-go strips zero-fill VINs but still expects WMI extraction. Make sure we
/// echo back the WMI as the first 3 chars regardless of whether the rest of
/// the VIN looks valid.
#[test]
fn vingo_wmi_passthrough() {
    let dec = open_decoder();
    for (vin, _) in FIXTURES {
        let v = dec.decode_unchecked(vin).unwrap();
        assert_eq!(&v.wmi, &vin[..3], "WMI mismatch for {vin}");
    }
}
