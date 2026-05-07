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
        Some(match s {
            "Make" => Element::Make,
            "Model" => Element::Model,
            "Series" => Element::Series,
            "Trim" | "TrimLevel" => Element::Trim,
            "BodyClass" | "BodyStyle" => Element::BodyClass,
            "FuelTypePrimary" => Element::FuelTypePrimary,
            "FuelTypeSecondary" => Element::FuelTypeSecondary,
            "Doors" => Element::Doors,
            "EngineCylinders" => Element::EngineCylinders,
            "EngineModel" => Element::EngineModel,
            "EngineConfiguration" => Element::EngineConfiguration,
            "EngineManufacturer" => Element::EngineManufacturer,
            "DisplacementL" => Element::DisplacementL,
            "Turbo" => Element::Turbo,
            "DriveType" => Element::DriveType,
            "Transmission" | "TransmissionStyle" => Element::Transmission,
            "BatteryType" => Element::BatteryType,
            "ChargerLevel" => Element::ChargerLevel,
            "EVDriveUnit" => Element::EvDriveUnit,
            "BrakeSystemType" => Element::BrakeSystemType,
            "GVWR" => Element::Gvwr,
            "PlantCity" => Element::PlantCity,
            "PlantState" => Element::PlantState,
            "PlantCountry" => Element::PlantCountry,
            "Manufacturer" | "ManufacturerName" => Element::Manufacturer,
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
        match self {
            Element::Make => vehicle.make = Some(value),
            Element::Model => vehicle.model = Some(value),
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
            Element::Manufacturer => vehicle.manufacturer = Some(value),
        }
    }
}
