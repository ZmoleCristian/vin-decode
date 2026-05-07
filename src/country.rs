//! ISO 3779 country code → country name mapping.
//!
//! VIN positions 1-2 encode a country range. This is *not* exhaustive — many
//! prefix ranges aren't assigned — and it returns `None` for unknown codes so
//! the caller can fall back to plant_country from the WMI metadata table.

/// Map a 2-char country code (VIN positions 1-2) to a country name.
///
/// Recognises the prefixes covered by the test fixtures we ship: EU, North
/// America, key Asian and South American manufacturers. For unmapped ranges,
/// returns `None`.
pub fn country_from_code(code: &str) -> Option<&'static str> {
    if code.len() != 2 {
        return None;
    }
    let bytes = code.as_bytes();
    let c0 = bytes[0] as char;
    let c1 = bytes[1] as char;

    // North America
    match c0 {
        '1' | '4' | '5' => return Some("United States"),
        '2' => return Some("Canada"),
        '3' => {
            return Some(match c1 {
                'A'..='W' => "Mexico",
                _ => "Costa Rica",
            });
        }
        // Oceania
        '6' => return Some("Australia"),
        '7' => return Some("New Zealand"),
        // South America
        '8' => {
            return Some(match c1 {
                'A'..='E' => "Argentina",
                'F'..='K' => "Chile",
                'L'..='R' => "Ecuador",
                'S'..='W' => "Peru",
                'X'..='Z' => "Venezuela",
                _ => "South America",
            });
        }
        '9' => return Some("Brazil"),
        _ => {}
    }

    // Africa
    if let 'A'..='H' = c0 {
        return Some(match c0 {
            'A' => match c1 {
                'A'..='H' => "South Africa",
                _ => "Ivory Coast",
            },
            'B' => match c1 {
                'A'..='E' => "Angola",
                'F'..='K' => "Kenya",
                _ => "Tanzania",
            },
            'C' => match c1 {
                'A'..='E' => "Benin",
                'F'..='K' => "Madagascar",
                _ => "Tunisia",
            },
            'D' => match c1 {
                'A'..='E' => "Egypt",
                'F'..='K' => "Morocco",
                _ => "Zambia",
            },
            _ => "Africa",
        });
    }

    // Asia
    if let 'J'..='R' = c0 {
        return Some(match c0 {
            'J' => "Japan",
            'K' => match c1 {
                'A'..='E' => "Sri Lanka",
                'F'..='K' => "Israel",
                'L'..='R' => "South Korea",
                _ => "Kazakhstan",
            },
            'L' => "China",
            'M' => match c1 {
                'A'..='E' => "India",
                'F'..='K' => "Indonesia",
                'L'..='R' => "Thailand",
                _ => "Myanmar",
            },
            'N' => match c1 {
                'A'..='E' => "Iran",
                'F'..='K' => "Pakistan",
                _ => "Turkey",
            },
            'P' => match c1 {
                'A'..='E' => "Philippines",
                'F'..='K' => "Singapore",
                _ => "Malaysia",
            },
            'R' => match c1 {
                'A'..='E' => "United Arab Emirates",
                'F'..='K' => "Taiwan",
                _ => "Vietnam",
            },
            _ => "Asia",
        });
    }

    // Europe (S-Z)
    if let 'S'..='Z' = c0 {
        return Some(match c0 {
            'S' => match c1 {
                'A'..='M' => "United Kingdom",
                'N'..='T' => "Germany",
                'U'..='Z' => "Poland",
                _ => "Latvia",
            },
            'T' => match c1 {
                'A'..='H' => "Switzerland",
                'J'..='P' => "Czech Republic",
                'R'..='V' => "Hungary",
                _ => "Portugal",
            },
            'U' => match c1 {
                'H'..='M' => "Denmark",
                'N'..='T' => "Ireland",
                'U'..='Z' => "Romania",
                _ => "Slovakia",
            },
            'V' => match c1 {
                'A'..='E' => "Austria",
                'F'..='R' => "France",
                'S'..='W' => "Spain",
                _ => "Yugoslavia/Serbia",
            },
            'W' => "Germany",
            'X' => match c1 {
                'A'..='E' => "Bulgaria",
                'F'..='K' => "Greece",
                'L'..='R' => "Netherlands",
                _ => "Russia",
            },
            'Y' => match c1 {
                'A'..='E' => "Belgium",
                'F'..='K' => "Finland",
                'L'..='R' => "Malta",
                'S'..='W' => "Sweden",
                _ => "Norway",
            },
            'Z' => match c1 {
                'A'..='R' => "Italy",
                _ => "Slovenia",
            },
            _ => "Europe",
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vininfo_fixtures() {
        assert_eq!(country_from_code("95"), Some("Brazil"));
        assert_eq!(country_from_code("92"), Some("Brazil"));
        assert_eq!(country_from_code("MD"), Some("India"));
        assert_eq!(country_from_code("XT"), Some("Russia"));
        assert_eq!(country_from_code("W0"), Some("Germany"));
        assert_eq!(country_from_code("WV"), Some("Germany"));
        assert_eq!(country_from_code("VF"), Some("France"));
        assert_eq!(country_from_code("5N"), Some("United States"));
        assert_eq!(country_from_code("6F"), Some("Australia"));
        assert_eq!(country_from_code("JS"), Some("Japan"));
        assert_eq!(country_from_code("TM"), Some("Czech Republic"));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(country_from_code(""), None);
        assert_eq!(country_from_code("A"), None);
        assert_eq!(country_from_code("ABC"), None);
        // '0' is reserved/unassigned
        assert_eq!(country_from_code("0A"), None);
    }
}
