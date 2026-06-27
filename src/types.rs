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

    /// Both possible model-year candidates from the VIN's pos-10 year code.
    ///
    /// SAE-J853 reuses each letter twice (30-year cycle), so a code like
    /// `'F'` maps to both 1985 and 2015. This returns both. Numeric codes
    /// `1`-`9` only ever map to a single year (2001-2009) since the second
    /// digit cycle (2031-2039) hasn't started. Returns an empty vec for
    /// unreadable codes (`I`/`O`/`Q`/`U`/`Z`/`0`).
    ///
    /// Note: many manufacturers don't follow the SAE-J853 pos-10 convention
    /// (modern Mercedes encodes year in the chassis serial; some Renault /
    /// Dacia families use other positions; Ford EU uses pos-11). For those
    /// VINs this method may return candidates that don't include the actual
    /// model year. The decoder no longer auto-picks one — consumers can
    /// inspect the candidates and make their own call.
    pub fn year_candidates(&self) -> Vec<u32> {
        let code = self.year_code();
        let Some(base) = crate::year::year_for_code(code) else {
            return Vec::new();
        };
        if code.is_ascii_digit() {
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
    /// Bodywork type, per the EU type-approval standard (see [`BodyType`]).
    pub body_type: Option<BodyType>,
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

/// Vehicle bodywork type, per the EU type-approval standard — the two-letter
/// codes from Commission Regulation (EU) 678/2011 (consolidated into the
/// framework Reg (EU) 2018/858), the same codes printed on the Certificate of
/// Conformity and recorded by EU national vehicle registers.
///
/// This is a *bodywork* classification, not a market segment. There is
/// deliberately **no `SUV` / `Crossover`** variant: the standard has none — an
/// SUV is type-approved as a [`BodyType::StationWagon`] (`AC`) or
/// [`BodyType::MultiPurpose`] (`AF`). Segment labelling (SUV, city car, …) is
/// an application-layer concern and is intentionally out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum BodyType {
    // Category M1 — passenger cars
    Saloon,            // AA
    Hatchback,         // AB
    StationWagon,      // AC
    Coupe,             // AD
    Convertible,       // AE
    MultiPurpose,      // AF
    TruckStationWagon, // AG
    // Category N — goods vehicles
    Lorry,       // BA
    Van,         // BB
    TractorUnit, // BC
    RoadTractor, // BD
    PickUp,      // BE
    ChassisCab,  // BX
    // Category M2/M3 — buses & coaches
    SingleDeckBus,                    // CA
    DoubleDeckBus,                    // CB
    ArticulatedSingleDeckBus,         // CC
    ArticulatedDoubleDeckBus,         // CD
    LowFloorSingleDeckBus,            // CE
    LowFloorDoubleDeckBus,            // CF
    ArticulatedLowFloorSingleDeckBus, // CG
    ArticulatedLowFloorDoubleDeckBus, // CH
    OpenTopSingleDeckBus,             // CI
    OpenTopDoubleDeckBus,             // CJ
    BusChassis,                       // CX
    // Category O — trailers
    SemiTrailer,         // DA
    DrawbarTrailer,      // DB
    CentreAxleTrailer,   // DC
    RigidDrawbarTrailer, // DE
    // Special-purpose vehicles
    MotorCaravan,           // SA
    ArmouredVehicle,        // SB
    Ambulance,              // SC
    Hearse,                 // SD
    TrailerCaravan,         // SE
    MobileCrane,            // SF
    SpecialGroup,           // SG
    WheelchairAccessible,   // SH
    ConverterDolly,         // SJ
    ExceptionalLoadTrailer, // SK
    /// Bodywork that maps to no EU type-approval code — e.g. a category L
    /// powered two-wheeler, or an unrecognised source string.
    Unknown,
}

/// Every standard (code-bearing) variant, in EU-code order. Excludes
/// [`BodyType::Unknown`], which is the absence of a code rather than a type.
pub(crate) const BODY_TYPES: &[BodyType] = &[
    BodyType::Saloon,
    BodyType::Hatchback,
    BodyType::StationWagon,
    BodyType::Coupe,
    BodyType::Convertible,
    BodyType::MultiPurpose,
    BodyType::TruckStationWagon,
    BodyType::Lorry,
    BodyType::Van,
    BodyType::TractorUnit,
    BodyType::RoadTractor,
    BodyType::PickUp,
    BodyType::ChassisCab,
    BodyType::SingleDeckBus,
    BodyType::DoubleDeckBus,
    BodyType::ArticulatedSingleDeckBus,
    BodyType::ArticulatedDoubleDeckBus,
    BodyType::LowFloorSingleDeckBus,
    BodyType::LowFloorDoubleDeckBus,
    BodyType::ArticulatedLowFloorSingleDeckBus,
    BodyType::ArticulatedLowFloorDoubleDeckBus,
    BodyType::OpenTopSingleDeckBus,
    BodyType::OpenTopDoubleDeckBus,
    BodyType::BusChassis,
    BodyType::SemiTrailer,
    BodyType::DrawbarTrailer,
    BodyType::CentreAxleTrailer,
    BodyType::RigidDrawbarTrailer,
    BodyType::MotorCaravan,
    BodyType::ArmouredVehicle,
    BodyType::Ambulance,
    BodyType::Hearse,
    BodyType::TrailerCaravan,
    BodyType::MobileCrane,
    BodyType::SpecialGroup,
    BodyType::WheelchairAccessible,
    BodyType::ConverterDolly,
    BodyType::ExceptionalLoadTrailer,
];

impl BodyType {
    /// The canonical two-letter EU type-approval bodywork code (`"AA"`, `"BB"`,
    /// …). [`BodyType::Unknown`] has no code and returns `""`.
    pub fn code(self) -> &'static str {
        match self {
            BodyType::Saloon => "AA",
            BodyType::Hatchback => "AB",
            BodyType::StationWagon => "AC",
            BodyType::Coupe => "AD",
            BodyType::Convertible => "AE",
            BodyType::MultiPurpose => "AF",
            BodyType::TruckStationWagon => "AG",
            BodyType::Lorry => "BA",
            BodyType::Van => "BB",
            BodyType::TractorUnit => "BC",
            BodyType::RoadTractor => "BD",
            BodyType::PickUp => "BE",
            BodyType::ChassisCab => "BX",
            BodyType::SingleDeckBus => "CA",
            BodyType::DoubleDeckBus => "CB",
            BodyType::ArticulatedSingleDeckBus => "CC",
            BodyType::ArticulatedDoubleDeckBus => "CD",
            BodyType::LowFloorSingleDeckBus => "CE",
            BodyType::LowFloorDoubleDeckBus => "CF",
            BodyType::ArticulatedLowFloorSingleDeckBus => "CG",
            BodyType::ArticulatedLowFloorDoubleDeckBus => "CH",
            BodyType::OpenTopSingleDeckBus => "CI",
            BodyType::OpenTopDoubleDeckBus => "CJ",
            BodyType::BusChassis => "CX",
            BodyType::SemiTrailer => "DA",
            BodyType::DrawbarTrailer => "DB",
            BodyType::CentreAxleTrailer => "DC",
            BodyType::RigidDrawbarTrailer => "DE",
            BodyType::MotorCaravan => "SA",
            BodyType::ArmouredVehicle => "SB",
            BodyType::Ambulance => "SC",
            BodyType::Hearse => "SD",
            BodyType::TrailerCaravan => "SE",
            BodyType::MobileCrane => "SF",
            BodyType::SpecialGroup => "SG",
            BodyType::WheelchairAccessible => "SH",
            BodyType::ConverterDolly => "SJ",
            BodyType::ExceptionalLoadTrailer => "SK",
            BodyType::Unknown => "",
        }
    }

    /// Map a two-letter EU type-approval code (case-insensitive) onto a variant.
    /// Returns `None` for an unrecognised code. This is the exact path for
    /// registry data that already carries the standard code (e.g. RDW
    /// `carrosserietype`).
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code.trim().to_ascii_uppercase().as_str() {
            "AA" => BodyType::Saloon,
            "AB" => BodyType::Hatchback,
            "AC" => BodyType::StationWagon,
            "AD" => BodyType::Coupe,
            "AE" => BodyType::Convertible,
            "AF" => BodyType::MultiPurpose,
            "AG" => BodyType::TruckStationWagon,
            "BA" => BodyType::Lorry,
            "BB" => BodyType::Van,
            "BC" => BodyType::TractorUnit,
            "BD" => BodyType::RoadTractor,
            "BE" => BodyType::PickUp,
            "BX" => BodyType::ChassisCab,
            "CA" => BodyType::SingleDeckBus,
            "CB" => BodyType::DoubleDeckBus,
            "CC" => BodyType::ArticulatedSingleDeckBus,
            "CD" => BodyType::ArticulatedDoubleDeckBus,
            "CE" => BodyType::LowFloorSingleDeckBus,
            "CF" => BodyType::LowFloorDoubleDeckBus,
            "CG" => BodyType::ArticulatedLowFloorSingleDeckBus,
            "CH" => BodyType::ArticulatedLowFloorDoubleDeckBus,
            "CI" => BodyType::OpenTopSingleDeckBus,
            "CJ" => BodyType::OpenTopDoubleDeckBus,
            "CX" => BodyType::BusChassis,
            "DA" => BodyType::SemiTrailer,
            "DB" => BodyType::DrawbarTrailer,
            "DC" => BodyType::CentreAxleTrailer,
            "DE" => BodyType::RigidDrawbarTrailer,
            "SA" => BodyType::MotorCaravan,
            "SB" => BodyType::ArmouredVehicle,
            "SC" => BodyType::Ambulance,
            "SD" => BodyType::Hearse,
            "SE" => BodyType::TrailerCaravan,
            "SF" => BodyType::MobileCrane,
            "SG" => BodyType::SpecialGroup,
            "SH" => BodyType::WheelchairAccessible,
            "SJ" => BodyType::ConverterDolly,
            "SK" => BodyType::ExceptionalLoadTrailer,
            _ => return None,
        })
    }

    /// Best-effort map of a free-text body-style string — vPIC English, RDW
    /// Dutch `inrichting`, autoevolution/DBpedia prose — onto the standard
    /// variant. Order matters: more specific substrings win first.
    ///
    /// Market-segment words with no bodywork code (`"SUV"`, `"crossover"`,
    /// `"CUV"`) collapse to [`BodyType::MultiPurpose`] (`AF`), the standard's
    /// closest catch-all for a tall multi-use passenger car. Genuinely
    /// unrecognised input yields [`BodyType::Unknown`].
    pub fn parse(s: &str) -> Self {
        let lc = s.to_ascii_lowercase();
        let has = |needle: &str| lc.contains(needle);
        // Special-purpose (most specific first).
        if has("ambulance") {
            return BodyType::Ambulance;
        }
        if has("hearse") || has("lijkwagen") {
            return BodyType::Hearse;
        }
        if has("motor caravan") || has("motorhome") || has("camper") {
            return BodyType::MotorCaravan;
        }
        if has("armoured") || has("armored") {
            return BodyType::ArmouredVehicle;
        }
        if has("wheelchair") {
            return BodyType::WheelchairAccessible;
        }
        if has("crane") {
            return BodyType::MobileCrane;
        }
        // Trailers & buses.
        if has("semi-trailer") || has("semitrailer") {
            return BodyType::SemiTrailer;
        }
        if has("trailer") {
            return BodyType::DrawbarTrailer;
        }
        if has("double") && (has("deck") || has("decker")) {
            return BodyType::DoubleDeckBus;
        }
        if has("bus") || has("coach") {
            return BodyType::SingleDeckBus;
        }
        // Goods vehicles.
        if has("pick-up") || has("pickup") || has("pick up") {
            return BodyType::PickUp;
        }
        if has("tractor unit") || has("semi tractor") {
            return BodyType::TractorUnit;
        }
        if has("road tractor") {
            return BodyType::RoadTractor;
        }
        if has("chassis") {
            return BodyType::ChassisCab;
        }
        // Passenger cars.
        if has("truck station wagon") {
            return BodyType::TruckStationWagon;
        }
        if has("multi-purpose")
            || has("multipurpose")
            || has("mpv")
            || has("minivan")
            || has("people carrier")
            || has("monovolume")
            // Market-segment words with no bodywork code — closest standard bucket.
            || has("suv")
            || has("sport utility")
            || has("crossover")
            || has("cuv")
            || has("off-road")
        {
            return BodyType::MultiPurpose;
        }
        if has("station wagon")
            || has("stationwagen")
            || has("estate")
            || has("wagon")
            || has("kombi")
            || has("combi")
            || has("touring")
            || has("avant")
            || has("variant")
            || has("break")
        {
            return BodyType::StationWagon;
        }
        if has("convertible")
            || has("cabrio")
            || has("roadster")
            || has("spider")
            || has("spyder")
            || has("targa")
            || has("drophead")
        {
            return BodyType::Convertible;
        }
        if has("coupe") || has("coupé") {
            return BodyType::Coupe;
        }
        if has("hatchback")
            || has("hatch")
            || has("liftback")
            || has("fastback")
            || has("sportback")
        {
            return BodyType::Hatchback;
        }
        if has("van") || has("gesloten opbouw") {
            return BodyType::Van;
        }
        if has("saloon")
            || has("sedan")
            || has("berlina")
            || has("berline")
            || has("limousine")
            || has("notchback")
        {
            return BodyType::Saloon;
        }
        // Generic "truck" / "lorry" last so "truck station wagon" etc. win above.
        if has("lorry") || has("truck") {
            return BodyType::Lorry;
        }
        BodyType::Unknown
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

/// Drive layout, normalised from the free-text `drive` strings in the engine
/// catalog (`"Front Wheel Drive"`, `"All Wheel Drive"`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum DriveType {
    Fwd,
    Rwd,
    Awd,
    FourWheelDrive,
    Other,
}

impl DriveType {
    /// Parse a free-form drive-layout string into a variant.
    pub fn parse(s: &str) -> Self {
        let lc = s.to_ascii_lowercase();
        if lc.contains("front") {
            DriveType::Fwd
        } else if lc.contains("rear") {
            DriveType::Rwd
        } else if lc.contains("all wheel") || lc.contains("all-wheel") || lc.trim() == "awd" {
            DriveType::Awd
        } else if lc.contains("four wheel")
            || lc.contains("four-wheel")
            || lc.contains("4wd")
            || lc.contains("4x4")
        {
            DriveType::FourWheelDrive
        } else {
            DriveType::Other
        }
    }
}

/// Transmission type, normalised from the free-text `gearbox` strings in the
/// engine catalog (`"6-Speed Manual"`, `"7-Speed Dual Clutch"`, `"CVT"`, …).
/// Gear count is separate — see [`crate::EngineRow::gearbox_speeds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub enum Transmission {
    Manual,
    Automatic,
    DualClutch,
    Cvt,
    SingleSpeed,
    AutomatedManual,
    Other,
}

impl Transmission {
    /// Parse a free-form gearbox string into a variant. Order matters: specific
    /// box types (CVT, dual-clutch, automated-manual) are tested before the
    /// generic `"manual"` / `"automatic"` substrings.
    pub fn parse(s: &str) -> Self {
        let lc = s.to_ascii_lowercase();
        if lc.contains("cvt") || lc.contains("continuously variable") {
            Transmission::Cvt
        } else if lc.contains("single-speed") || lc.contains("single speed") || lc.contains("1-speed")
        {
            Transmission::SingleSpeed
        } else if lc.contains("dual clutch")
            || lc.contains("dual-clutch")
            || lc.contains("twin clutch")
            || lc.contains("dsg")
            || lc.contains("dct")
            || lc.contains("dkg")
            || lc.contains("pdk")
            || lc.contains("s tronic")
            || lc.contains("s-tronic")
            || lc.contains("powershift")
        {
            Transmission::DualClutch
        } else if lc.contains("automated manual")
            || lc.contains("automatic manual")
            || lc.contains("automatized")
            || lc.contains("semi-automatic")
            || lc.contains("amt")
        {
            Transmission::AutomatedManual
        } else if lc.contains("manual") {
            Transmission::Manual
        } else if lc.contains("automatic") || lc.contains("auto") || lc.contains("tiptronic") {
            Transmission::Automatic
        } else {
            Transmission::Other
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
    fn body_type_parse_standard_mappings() {
        // vPIC English
        assert_eq!(BodyType::parse("4-Door Sedan"), BodyType::Saloon);
        assert_eq!(BodyType::parse("2-Door Coupe"), BodyType::Coupe);
        assert_eq!(BodyType::parse("Hatchback"), BodyType::Hatchback);
        assert_eq!(BodyType::parse("Station Wagon"), BodyType::StationWagon);
        assert_eq!(BodyType::parse("Convertible"), BodyType::Convertible);
        assert_eq!(BodyType::parse("2-Door Cabriolet"), BodyType::Convertible);
        assert_eq!(BodyType::parse("Roadster"), BodyType::Convertible);
        assert_eq!(BodyType::parse("Crew Cab Pickup"), BodyType::PickUp);
        assert_eq!(BodyType::parse("Cargo Van"), BodyType::Van);
        // EU / continental / registry vocab
        assert_eq!(BodyType::parse("Berline"), BodyType::Saloon);
        assert_eq!(BodyType::parse("Kombi"), BodyType::StationWagon);
        assert_eq!(BodyType::parse("Avant"), BodyType::StationWagon);
        assert_eq!(BodyType::parse("stationwagen"), BodyType::StationWagon);
        assert_eq!(BodyType::parse("gesloten opbouw"), BodyType::Van);
        // MPV + market-segment words (no bodywork code) collapse to AF
        assert_eq!(BodyType::parse("Minivan"), BodyType::MultiPurpose);
        assert_eq!(
            BodyType::parse("Sport Utility Vehicle (SUV)"),
            BodyType::MultiPurpose
        );
        assert_eq!(
            BodyType::parse("Crossover Utility Vehicle (CUV)"),
            BodyType::MultiPurpose
        );
        // special-purpose
        assert_eq!(BodyType::parse("Ambulance"), BodyType::Ambulance);
        assert_eq!(BodyType::parse("Hearse"), BodyType::Hearse);
        // category L two-wheelers / junk have no bodywork code
        assert_eq!(BodyType::parse("Motorcycle"), BodyType::Unknown);
        assert_eq!(BodyType::parse("Unknown blob"), BodyType::Unknown);
    }

    #[test]
    fn body_type_code_roundtrip() {
        for &b in BODY_TYPES {
            assert_eq!(BodyType::from_code(b.code()), Some(b), "roundtrip {b:?}");
        }
        assert_eq!(BODY_TYPES.len(), 38);
        assert_eq!(BodyType::from_code("aa"), Some(BodyType::Saloon)); // case-insensitive
        assert_eq!(BodyType::from_code("ZZ"), None);
        assert_eq!(BodyType::Unknown.code(), "");
    }

    #[test]
    fn drive_type_parse() {
        assert_eq!(DriveType::parse("Front Wheel Drive"), DriveType::Fwd);
        assert_eq!(DriveType::parse("Rear Wheel Drive"), DriveType::Rwd);
        assert_eq!(DriveType::parse("All Wheel Drive"), DriveType::Awd);
        assert_eq!(
            DriveType::parse("Four Wheel Drive"),
            DriveType::FourWheelDrive
        );
        assert_eq!(DriveType::parse("4x4"), DriveType::FourWheelDrive);
        assert_eq!(DriveType::parse(""), DriveType::Other);
    }

    #[test]
    fn transmission_parse() {
        assert_eq!(Transmission::parse("6-Speed Manual"), Transmission::Manual);
        assert_eq!(
            Transmission::parse("7-Speed Automatic"),
            Transmission::Automatic
        );
        assert_eq!(
            Transmission::parse("7-Speed Dual Clutch"),
            Transmission::DualClutch
        );
        assert_eq!(Transmission::parse("6-Speed DSG"), Transmission::DualClutch);
        assert_eq!(Transmission::parse("7-Speed S tronic"), Transmission::DualClutch);
        assert_eq!(Transmission::parse("CVT"), Transmission::Cvt);
        assert_eq!(
            Transmission::parse("Single-Speed"),
            Transmission::SingleSpeed
        );
        assert_eq!(
            Transmission::parse("5-Speed Automated Manual"),
            Transmission::AutomatedManual
        );
        assert_eq!(Transmission::parse("Tiptronic"), Transmission::Automatic);
        assert_eq!(Transmission::parse(""), Transmission::Other);
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
