//! vPIC element-name → typed enum dispatch.

use crate::types::{BodyClass, FuelType, Vehicle};

/// Subset of vPIC element codes that we surface in [`Vehicle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Element {
    Make,
    Model,
    Series,
    Trim,
    BodyClass,
    FuelTypePrimary,
    FuelTypeSecondary,
    Doors,
    EngineCylinders,
    EngineModel,
    EngineConfiguration,
    EngineManufacturer,
    DisplacementL,
    Turbo,
    DriveType,
    Transmission,
    BatteryType,
    ChargerLevel,
    EvDriveUnit,
    BrakeSystemType,
    Gvwr,
    PlantCity,
    PlantState,
    PlantCountry,
    Manufacturer,
}

impl Element {
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        // vPIC's Element.Name field uses spaces and human-readable forms
        // ("Body Class" not "BodyClass"). Accept both — our build pipeline
        // emits Element.Name verbatim, but tests/fixtures may use the camelCase.
        Some(match s {
            "Make" | "NCSA Make" => Element::Make,
            "Model" | "NCSA Model" => Element::Model,
            "Series" => Element::Series,
            "Trim" | "TrimLevel" | "Trim2" => Element::Trim,
            "BodyClass" | "Body Class" | "BodyStyle" => Element::BodyClass,
            "FuelTypePrimary" | "Fuel Type - Primary" => Element::FuelTypePrimary,
            "FuelTypeSecondary" | "Fuel Type - Secondary" => Element::FuelTypeSecondary,
            "Doors" => Element::Doors,
            "EngineCylinders" | "Engine Number of Cylinders" => Element::EngineCylinders,
            "EngineModel" | "Engine Model" => Element::EngineModel,
            "EngineConfiguration" | "Engine Configuration" => Element::EngineConfiguration,
            "EngineManufacturer" | "Engine Manufacturer" => Element::EngineManufacturer,
            "DisplacementL" | "Displacement (L)" => Element::DisplacementL,
            "Turbo" => Element::Turbo,
            "DriveType" | "Drive Type" => Element::DriveType,
            "Transmission" | "TransmissionStyle" | "Transmission Style" => Element::Transmission,
            "BatteryType" | "Battery Type" => Element::BatteryType,
            "ChargerLevel" | "Charger Level" => Element::ChargerLevel,
            "EVDriveUnit" | "EV Drive Unit" | "Electrification Level" => Element::EvDriveUnit,
            "BrakeSystemType" | "Brake System Type" => Element::BrakeSystemType,
            "GVWR" | "Gross Vehicle Weight Rating From" | "Gross Vehicle Weight Rating To" => {
                Element::Gvwr
            }
            "PlantCity" | "Plant City" => Element::PlantCity,
            "PlantState" | "Plant State" => Element::PlantState,
            "PlantCountry" | "Plant Country" => Element::PlantCountry,
            "Manufacturer" | "ManufacturerName" | "Manufacturer Name" => Element::Manufacturer,
            _ => return None,
        })
    }

    pub(crate) fn confidence_cutoff(self) -> f64 {
        match self {
            Element::PlantCity | Element::PlantState | Element::PlantCountry => 0.3,
            _ => 0.5,
        }
    }

    pub(crate) fn apply(self, vehicle: &mut Vehicle, value: String) {
        // Defensive guard: vPIC FK-typed elements occasionally arrive
        // unresolved (purely numeric strings — IDs that didn't match a lookup
        // table at extraction time). Don't let those clobber a properly-named
        // make/model/manufacturer that came from the WMI table.
        let looks_unresolved_fk =
            !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) && value.len() < 6;
        match self {
            Element::Make => {
                if !(looks_unresolved_fk && vehicle.make.is_some()) {
                    vehicle.make = Some(value);
                }
            }
            Element::Model => {
                if !looks_unresolved_fk {
                    vehicle.model = Some(value);
                }
            }
            Element::Manufacturer => {
                if !(looks_unresolved_fk && vehicle.manufacturer.is_some()) {
                    vehicle.manufacturer = Some(value);
                }
            }
            Element::Series => vehicle.series = Some(value),
            Element::Trim => vehicle.trim = Some(value),
            Element::BodyClass => vehicle.body_class = Some(BodyClass::parse(&value)),
            Element::FuelTypePrimary => vehicle.fuel_primary = Some(FuelType::parse(&value)),
            Element::FuelTypeSecondary => vehicle.fuel_secondary = Some(FuelType::parse(&value)),
            Element::Doors => vehicle.doors = value.parse().ok(),
            Element::EngineCylinders => vehicle.engine_cylinders = value.parse().ok(),
            Element::EngineModel => vehicle.engine_model = Some(value),
            Element::EngineConfiguration => vehicle.engine_configuration = Some(value),
            Element::EngineManufacturer => vehicle.engine_manufacturer = Some(value),
            Element::DisplacementL => vehicle.displacement_l = value.parse().ok(),
            Element::Turbo => vehicle.turbo = Some(matches!(value.as_str(), "Yes" | "yes" | "Y")),
            Element::DriveType => vehicle.drive_type = Some(value),
            Element::Transmission => vehicle.transmission = Some(value),
            Element::BatteryType => vehicle.battery_type = Some(value),
            Element::ChargerLevel => vehicle.charger_level = Some(value),
            Element::EvDriveUnit => vehicle.ev_drive_unit = Some(value),
            Element::BrakeSystemType => vehicle.brake_system = Some(value),
            Element::Gvwr => vehicle.gvwr = Some(value),
            Element::PlantCity => vehicle.plant_city = Some(value),
            Element::PlantState => vehicle.plant_state = Some(value),
            Element::PlantCountry => vehicle.plant_country = Some(value),
        }
    }
}
