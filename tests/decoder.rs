mod common;

use tempfile::TempDir;
use vin_decode::{BodyClass, Decoder, Error, FuelType};

const HONDA: &str = "1HGCM82633A004352";
const FORD: &str = "2FTEF14H8TCA73155";
const TESLA: &str = "5YJ3E1EA0KF000316";

#[test]
fn decode_honda_civic_full_attrs() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let v = dec.decode(HONDA).unwrap();
    assert_eq!(v.wmi, "1HG");
    assert_eq!(v.make.as_deref(), Some("Honda"));
    assert_eq!(v.model.as_deref(), Some("Civic"));
    assert_eq!(v.model_year, Some(2003));
    assert_eq!(v.body_class, Some(BodyClass::Sedan));
    assert_eq!(v.fuel_primary, Some(FuelType::Gasoline));
    assert_eq!(v.doors, Some(4));
}

#[test]
fn decode_ford_truck() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let v = dec.decode(FORD).unwrap();
    assert_eq!(v.make.as_deref(), Some("Ford"));
    assert_eq!(v.model.as_deref(), Some("F-150"));
    assert_eq!(v.model_year, Some(1996));
    assert_eq!(v.body_class, Some(BodyClass::Pickup));
}

#[test]
fn decode_tesla_electric() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let v = dec.decode(TESLA).unwrap();
    assert_eq!(v.make.as_deref(), Some("Tesla"));
    assert_eq!(v.model.as_deref(), Some("Model 3"));
    assert_eq!(v.model_year, Some(2019));
    assert_eq!(v.fuel_primary, Some(FuelType::Electric));
}

#[test]
fn rejects_unknown_wmi() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let bogus = "ZZZCM82633A00435X";
    let r = dec.decode(bogus);
    assert!(matches!(
        r,
        Err(Error::UnknownWmi(_)) | Err(Error::BadCheckDigit { .. })
    ));
}

#[test]
fn rejects_bad_check_digit() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let bad = "1HGCM82613A004352";
    let r = dec.decode(bad);
    assert!(matches!(r, Err(Error::BadCheckDigit { .. })));
}

#[test]
fn rejects_invalid_length() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    assert!(matches!(dec.decode("SHORT"), Err(Error::InvalidLength(_))));
}

#[test]
fn rejects_forbidden_chars() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let bad = "1HGCM8I633A004352";
    assert!(matches!(dec.decode(bad), Err(Error::ForbiddenChar('I'))));
}

#[test]
fn decode_unchecked_skips_checksum() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let bad_chk = "1HGCM82613A004352";
    let v = dec.decode_unchecked(bad_chk).unwrap();
    assert_eq!(v.make.as_deref(), Some("Honda"));
}

#[test]
fn weight_resolves_competing_patterns() {
    let dir = TempDir::new().unwrap();
    common::build_fixture(dir.path());
    let dec = Decoder::open(dir.path()).unwrap();
    let v = dec.decode(HONDA).unwrap();
    assert_eq!(
        v.model.as_deref(),
        Some("Civic"),
        "specific model wins over none"
    );
}

#[test]
fn missing_data_dir_returns_missing_data_error() {
    let dir = TempDir::new().unwrap();
    let r = Decoder::open(dir.path());
    assert!(matches!(r, Err(Error::MissingData(_))));
}
