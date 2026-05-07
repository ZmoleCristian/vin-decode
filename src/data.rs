//! Internal rkyv archive types for the lookup tables.
//!
//! These types are exposed publicly only when the `build` feature is enabled,
//! since the build pipeline needs to construct them. End users of the decoder
//! never see these — they go through [`crate::Decoder`] / [`crate::Catalog`].

use rkyv::{
    Archive, Deserialize, Serialize,
    api::high::{HighDeserializer, HighSerializer},
    rancor::Error as RkyvError,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};

/// Marker trait for types that can be rkyv-deserialized via the high-level helpers.
pub trait RkyvDe<T>: Deserialize<T, HighDeserializer<RkyvError>> {}

/// Marker trait for types that can be rkyv-serialized via the high-level helpers.
pub trait RkyvSer:
    for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkyvError>>
{
}

/// Trait that maps a row type to its on-disk file name (sans extension).
pub trait Saveable {
    /// Base name for the `.fst`/`.bin` file pair on disk.
    fn base_name() -> &'static str;
}

/// Single make-name row, keyed by WMI in the `wmi_make` table.
///
/// Country and region are populated when the WMI was sourced from KBA / vPIC /
/// merged datasets that carried plant origin metadata. Empty string means the
/// upstream source did not record this field.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MakeRow {
    /// Make name as it appears in the source (e.g. `"Honda"`).
    pub name: String,
    /// Plant country (uppercased canonical form, e.g. `"GERMANY"`).
    pub country: String,
    /// Region bucket (`"EUROPE"`, `"NORTH_AMERICA"`, `"ASIA"`, `"OCEANIA"`,
    /// `"SOUTH_AMERICA"`, `"AFRICA"`).
    pub region: String,
}
impl RkyvSer for MakeRow {}
impl RkyvDe<MakeRow> for ArchivedMakeRow {}
impl Saveable for MakeRow {
    fn base_name() -> &'static str {
        "wmi_make"
    }
}

/// Brand → model index row with year range, sourced from the EU/global rip.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EuModelRow {
    /// Model name (uppercase canonical, e.g. `"FABIA"`, `"OCTAVIA RS"`).
    pub name: String,
    /// First production year known for this model. `0` if unknown.
    pub first_year: u16,
    /// Last production year known for this model. `0` if unknown.
    pub last_year: u16,
}
impl RkyvSer for EuModelRow {}
impl RkyvDe<EuModelRow> for ArchivedEuModelRow {}
impl Saveable for EuModelRow {
    fn base_name() -> &'static str {
        "eu_brand_models"
    }
}

/// Engine variant row, keyed by `BRAND` in the engine catalog. Filter by
/// `model` in user code (or use `Catalog::engines_for(brand, model)`).
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EngineRow {
    /// Model name this engine belongs to (e.g. `"FABIA"`, `"OCTAVIA RS"`).
    pub model: String,
    /// Production year of this engine variant.
    pub year: u16,
    /// Engine variant display name (e.g. `"2.0 TDI 150hp"`).
    pub name: String,
    /// Number of cylinders / configuration string (e.g. `"4"`, `"V6"`, `"I4"`).
    pub cylinders: String,
    /// Displacement in cubic centimetres.
    pub displacement_cm3: u32,
    /// Continuous power output in kilowatts.
    pub power_kw: u32,
    /// Continuous power output in metric horsepower.
    pub power_hp: u32,
    /// Peak torque in newton-metres.
    pub torque_nm: u32,
    /// Fuel name as published (e.g. `"Gasoline"`, `"Diesel"`, `"Electric"`).
    pub fuel: String,
    /// Drive type (e.g. `"Front Wheel Drive"`, `"All Wheel Drive"`).
    pub drive: String,
    /// Gearbox description (e.g. `"6-Speed Manual"`).
    pub gearbox: String,
}
impl RkyvSer for EngineRow {}
impl RkyvDe<EngineRow> for ArchivedEngineRow {}
impl Saveable for EngineRow {
    fn base_name() -> &'static str {
        "eu_engines"
    }
}

/// Schema identifier row, keyed by WMI in the `wmi_schema` table.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SchemaRow {
    /// Schema identifier string (vPIC schema_id column).
    pub id: String,
}
impl RkyvSer for SchemaRow {}
impl RkyvDe<SchemaRow> for ArchivedSchemaRow {}
impl Saveable for SchemaRow {
    fn base_name() -> &'static str {
        "wmi_schema"
    }
}

/// Single lookup row, keyed by schema id in the `schema_lookup` table.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LookupRow {
    /// VIN pattern with optional `|VIS` metadata suffix.
    pub pattern: String,
    /// vPIC element name (e.g. `"Model"`, `"BodyClass"`, `"FuelTypePrimary"`).
    pub element: String,
    /// Resolved attribute value to apply when this pattern matches.
    pub value: String,
    /// Element weight — higher wins when multiple patterns match the same element.
    pub weight: u32,
}
impl RkyvSer for LookupRow {}
impl RkyvDe<LookupRow> for ArchivedLookupRow {}
impl Saveable for LookupRow {
    fn base_name() -> &'static str {
        "schema_lookup"
    }
}

/// Single model-name row, keyed by uppercase make in the `make_models` reverse index.
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ModelRow {
    /// Model name as it appears in vPIC patterns.
    pub name: String,
}
impl RkyvSer for ModelRow {}
impl RkyvDe<ModelRow> for ArchivedModelRow {}
impl Saveable for ModelRow {
    fn base_name() -> &'static str {
        "make_models"
    }
}
