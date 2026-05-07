use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A validated 17-character VIN.
///
/// Construction enforces length, ASCII-alphanumeric chars, and the I/O/Q ban.
/// Check-digit validation is separate (see [`crate::Decoder::decode`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Vin(String);

impl Vin {
    /// Parse a raw VIN string. Uppercases ASCII; rejects bad length/chars.
    pub fn new(raw: impl Into<String>) -> crate::Result<Self> {
        let s = raw.into().to_ascii_uppercase();
        crate::wmi::validate_chars(&s)?;
        Ok(Vin(s))
    }

    /// Borrow as canonical (uppercase) string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// World Manufacturer Identifier — first 3 chars.
    pub fn wmi(&self) -> &str {
        &self.0[..3]
    }

    /// Vehicle Descriptor Section — chars 4-9.
    pub fn vds(&self) -> &str {
        &self.0[3..9]
    }

    /// Vehicle Identifier Section — chars 10-17.
    pub fn vis(&self) -> &str {
        &self.0[9..]
    }

    /// Check digit at position 9.
    pub fn check_digit(&self) -> char {
        self.0.as_bytes()[8] as char
    }

    /// Model-year code at position 10.
    pub fn year_code(&self) -> char {
        self.0.as_bytes()[9] as char
    }

    /// Plant code at position 11.
    pub fn plant_code(&self) -> char {
        self.0.as_bytes()[10] as char
    }

    /// Region code — first character (ISO 3779 region bucket).
    pub fn region_code(&self) -> char {
        self.0.as_bytes()[0] as char
    }

    /// Country code — first two characters (ISO 3779 country range).
    pub fn country_code(&self) -> &str {
        &self.0[..2]
    }

    /// Squish-VIN — the 10-char fingerprint used by some lookup tools:
    /// chars 1-8 + chars 10-11 (skipping the check digit at position 9).
    pub fn squish_vin(&self) -> String {
        let s = &self.0;
        let mut out = String::with_capacity(10);
        out.push_str(&s[..8]);
        out.push_str(&s[9..11]);
        out
    }

    /// Both possible model-year candidates from the VIN.
    ///
    /// SAE-J853 reuses each letter twice (30-year cycle). When position 7 is a
    /// letter, the post-2009 candidate is generally correct; when it's a digit,
    /// the pre-2010 candidate is correct. Returns an empty vec for unreadable
    /// year codes (`I`/`O`/`Q`/`U`/`Z`/`0`). Numeric codes (`1`-`9`) only ever
    /// map to a single year (2001-2009) since we haven't completed a second
    /// cycle on digit codes yet.
    pub fn year_candidates(&self) -> Vec<u32> {
        let Some(base) = crate::year::year_for_code(self.year_code()) else {
            return Vec::new();
        };
        if self.year_code().is_ascii_digit() {
            vec![base]
        } else {
            vec![base + 30, base]
        }
    }
}

impl fmt::Display for Vin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Fully decoded vehicle attributes derived from a single VIN.
///
/// Every field is `Option<_>` — vPIC coverage is uneven, especially for
/// non-US-market vehicles. Always check what you got before unwrapping.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Vehicle {
    /// Original VIN string (uppercase).
    pub vin: String,
    /// World Manufacturer Identifier (first 3 chars of the VIN).
    pub wmi: String,
    /// Make name (e.g. `"Honda"`).
    pub make: Option<String>,
    /// Model name (e.g. `"Civic"`).
    pub model: Option<String>,
    /// Series identifier (sometimes used as a finer model variant).
    pub series: Option<String>,
    /// Trim level / package.
    pub trim: Option<String>,
    /// Model year (1980-2039, decoded from year code + position-7 disambiguator).
    pub model_year: Option<u32>,
    /// Body style category.
    pub body_class: Option<BodyClass>,
    /// Primary fuel type.
    pub fuel_primary: Option<FuelType>,
    /// Secondary fuel type (set on hybrids, dual-fuel).
    pub fuel_secondary: Option<FuelType>,
    /// Door count.
    pub doors: Option<u8>,
    /// Engine cylinder count.
    pub engine_cylinders: Option<u8>,
    /// Engine model designation.
    pub engine_model: Option<String>,
    /// Engine configuration (e.g. `"V"`, `"In-Line"`).
    pub engine_configuration: Option<String>,
    /// Engine manufacturer.
    pub engine_manufacturer: Option<String>,
    /// Displacement in liters.
    pub displacement_l: Option<f32>,
    /// Whether the engine is turbocharged.
    pub turbo: Option<bool>,
    /// Drive type (e.g. `"FWD"`, `"AWD"`).
    pub drive_type: Option<String>,
    /// Transmission style.
    pub transmission: Option<String>,
    /// Battery type (EV / hybrid).
    pub battery_type: Option<String>,
    /// On-board charger level (EV).
    pub charger_level: Option<String>,
    /// EV drive unit configuration.
    pub ev_drive_unit: Option<String>,
    /// Brake system type.
    pub brake_system: Option<String>,
    /// Gross vehicle weight rating.
    pub gvwr: Option<String>,
    /// Plant country.
    pub plant_country: Option<String>,
    /// Plant city.
    pub plant_city: Option<String>,
    /// Plant state/province.
    pub plant_state: Option<String>,
    /// Manufacturer name (often differs from make for OEM/coachbuilders).
    pub manufacturer: Option<String>,
    /// Continental region derived from the first VIN character (Africa/Asia/Europe/etc).
    pub region: Option<String>,
}

/// Coarse body-style enumeration the decoder normalizes vPIC strings into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum BodyClass {
    Sedan,
    Coupe,
    Hatchback,
    Wagon,
    Convertible,
    Suv,
    Crossover,
    Pickup,
    Van,
    Minivan,
    Bus,
    Truck,
    Motorcycle,
    Trailer,
    Incomplete,
    Other,
}

impl BodyClass {
    /// Parse a free-form vPIC body-style string into one of the coarse enum variants.
    pub fn parse(s: &str) -> Self {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            x if x.contains("sedan") => BodyClass::Sedan,
            x if x.contains("coupe") => BodyClass::Coupe,
            x if x.contains("hatchback") => BodyClass::Hatchback,
            x if x.contains("wagon") => BodyClass::Wagon,
            x if x.contains("convertible") || x.contains("cabrio") || x.contains("roadster") => {
                BodyClass::Convertible
            }
            x if x.contains("crossover") || x.contains("cuv") => BodyClass::Crossover,
            x if x.contains("sport utility") || x.contains("suv") => BodyClass::Suv,
            x if x.contains("pickup") => BodyClass::Pickup,
            x if x.contains("minivan") => BodyClass::Minivan,
            x if x.contains("van") => BodyClass::Van,
            x if x.contains("bus") => BodyClass::Bus,
            x if x.contains("truck") => BodyClass::Truck,
            x if x.contains("motorcycle") || x.contains("motor") => BodyClass::Motorcycle,
            x if x.contains("trailer") => BodyClass::Trailer,
            x if x.contains("incomplete") => BodyClass::Incomplete,
            _ => BodyClass::Other,
        }
    }
}

/// Fuel-type enumeration the decoder normalizes vPIC strings into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum FuelType {
    Gasoline,
    Diesel,
    Electric,
    Hybrid,
    PluginHybrid,
    Ethanol,
    FlexFuel,
    Cng,
    Lng,
    Lpg,
    Hydrogen,
    FuelCell,
    Methanol,
    NaturalGas,
    Other,
}

impl FuelType {
    /// Parse a free-form vPIC fuel-type string into one of the enum variants.
    pub fn parse(s: &str) -> Self {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            x if x.contains("gasoline") => FuelType::Gasoline,
            x if x.contains("diesel") => FuelType::Diesel,
            x if x.contains("plug") => FuelType::PluginHybrid,
            x if x.contains("hybrid") => FuelType::Hybrid,
            x if x.contains("methanol") || x.contains("m85") => FuelType::Methanol,
            x if x.contains("e85") || x.contains("ethanol") => FuelType::Ethanol,
            x if x.contains("flex") || x.contains("ffv") => FuelType::FlexFuel,
            x if x.contains("cng") || x.contains("compressed natural") => FuelType::Cng,
            x if x.contains("lng") || x.contains("liquefied natural") => FuelType::Lng,
            x if x.contains("lpg") || x.contains("propane") => FuelType::Lpg,
            x if x.contains("fuel cell") => FuelType::FuelCell,
            x if x.contains("hydrogen") => FuelType::Hydrogen,
            x if x.contains("electric") => FuelType::Electric,
            x if x.contains("natural gas") => FuelType::NaturalGas,
            _ => FuelType::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vin_uppercases_input() {
        let v = Vin::new("1hgcm82633a004352").unwrap();
        assert_eq!(v.as_str(), "1HGCM82633A004352");
    }

    #[test]
    fn vin_section_accessors() {
        let v = Vin::new("1HGCM82633A004352").unwrap();
        assert_eq!(v.wmi(), "1HG");
        assert_eq!(v.vds(), "CM8263");
        assert_eq!(v.vis(), "3A004352");
        assert_eq!(v.check_digit(), '3');
        assert_eq!(v.year_code(), '3');
        assert_eq!(v.plant_code(), 'A');
    }

    #[test]
    fn vin_display_returns_canonical() {
        let v = Vin::new("1hgcm82633a004352").unwrap();
        assert_eq!(format!("{}", v), "1HGCM82633A004352");
    }

    #[test]
    fn body_class_full_coverage() {
        assert_eq!(BodyClass::parse("4-Door Sedan"), BodyClass::Sedan);
        assert_eq!(BodyClass::parse("2-Door Coupe"), BodyClass::Coupe);
        assert_eq!(BodyClass::parse("Hatchback"), BodyClass::Hatchback);
        assert_eq!(BodyClass::parse("Station Wagon"), BodyClass::Wagon);
        assert_eq!(BodyClass::parse("Convertible"), BodyClass::Convertible);
        assert_eq!(BodyClass::parse("2-Door Cabriolet"), BodyClass::Convertible);
        assert_eq!(BodyClass::parse("Roadster"), BodyClass::Convertible);
        assert_eq!(
            BodyClass::parse("Crossover Utility Vehicle (CUV)"),
            BodyClass::Crossover
        );
        assert_eq!(
            BodyClass::parse("Sport Utility Vehicle (SUV)"),
            BodyClass::Suv
        );
        assert_eq!(BodyClass::parse("Crew Cab Pickup"), BodyClass::Pickup);
        assert_eq!(BodyClass::parse("Cargo Van"), BodyClass::Van);
        assert_eq!(BodyClass::parse("Minivan"), BodyClass::Minivan);
        assert_eq!(BodyClass::parse("School Bus"), BodyClass::Bus);
        assert_eq!(BodyClass::parse("Truck"), BodyClass::Truck);
        assert_eq!(BodyClass::parse("Motorcycle"), BodyClass::Motorcycle);
        assert_eq!(BodyClass::parse("Trailer"), BodyClass::Trailer);
        assert_eq!(
            BodyClass::parse("Incomplete Vehicle"),
            BodyClass::Incomplete
        );
        assert_eq!(BodyClass::parse("Unknown blob"), BodyClass::Other);
    }

    #[test]
    fn fuel_type_full_coverage() {
        assert_eq!(FuelType::parse("Gasoline"), FuelType::Gasoline);
        assert_eq!(FuelType::parse("Diesel"), FuelType::Diesel);
        assert_eq!(FuelType::parse("Electric"), FuelType::Electric);
        assert_eq!(FuelType::parse("Plug-in Hybrid"), FuelType::PluginHybrid);
        assert_eq!(FuelType::parse("Hybrid"), FuelType::Hybrid);
        assert_eq!(FuelType::parse("E85"), FuelType::Ethanol);
        assert_eq!(FuelType::parse("Ethanol (E85)"), FuelType::Ethanol);
        assert_eq!(
            FuelType::parse("Flexible Fuel Vehicle (FFV)"),
            FuelType::FlexFuel
        );
        assert_eq!(
            FuelType::parse("Compressed Natural Gas (CNG)"),
            FuelType::Cng
        );
        assert_eq!(
            FuelType::parse("Liquefied Natural Gas (LNG)"),
            FuelType::Lng
        );
        assert_eq!(
            FuelType::parse("Liquefied Petroleum Gas (LPG)"),
            FuelType::Lpg
        );
        assert_eq!(
            FuelType::parse("Compressed Hydrogen/Hydrogen"),
            FuelType::Hydrogen
        );
        assert_eq!(FuelType::parse("Fuel Cell"), FuelType::FuelCell);
        assert_eq!(FuelType::parse("Methanol (M85)"), FuelType::Methanol);
        assert_eq!(FuelType::parse("Unknown"), FuelType::Other);
    }
}
