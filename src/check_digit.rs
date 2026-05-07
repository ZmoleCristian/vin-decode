use crate::{Error, Vin};

const WEIGHTS: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];

fn translit(c: char) -> u32 {
    match c {
        '0'..='9' => c.to_digit(10).unwrap(),
        'A' | 'J' => 1,
        'B' | 'K' | 'S' => 2,
        'C' | 'L' | 'T' => 3,
        'D' | 'M' | 'U' => 4,
        'E' | 'N' | 'V' => 5,
        'F' | 'W' => 6,
        'G' | 'P' | 'X' => 7,
        'H' | 'Y' => 8,
        'R' | 'Z' => 9,
        _ => 0,
    }
}

pub fn validate(vin: &Vin) -> crate::Result<()> {
    let bytes = vin.as_str().as_bytes();
    let sum: u32 = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| translit(*b as char) * WEIGHTS[i])
        .sum();
    let expected = match sum % 11 {
        10 => 'X',
        n => char::from_digit(n, 10).unwrap(),
    };
    let actual = bytes[8] as char;
    if expected == actual {
        Ok(())
    } else {
        Err(Error::BadCheckDigit { expected, actual })
    }
}

#[allow(dead_code)]
pub fn compute(vin: &str) -> Option<char> {
    if vin.len() != 17 {
        return None;
    }
    let bytes = vin.as_bytes();
    let sum: u32 = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| translit(*b as char) * WEIGHTS[i])
        .sum();
    Some(match sum % 11 {
        10 => 'X',
        n => char::from_digit(n, 10).unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_valid_vins_pass() {
        let vins = [
            "1HGCM82633A004352",
            "2FTEF14H8TCA73155",
            "5YJ3E1EA0KF000316",
        ];
        for v in vins {
            let vin = Vin::new(v).unwrap();
            assert!(validate(&vin).is_ok(), "{} should validate", v);
        }
    }

    #[test]
    fn check_digit_x_for_remainder_10() {
        let bad = "1HGCM82633A004353";
        let vin = Vin::new(bad).unwrap();
        let r = validate(&vin);
        assert!(matches!(r, Err(Error::BadCheckDigit { .. })));
    }

    #[test]
    fn translit_letter_groups() {
        assert_eq!(translit('A'), 1);
        assert_eq!(translit('J'), 1);
        assert_eq!(translit('B'), 2);
        assert_eq!(translit('S'), 2);
        assert_eq!(translit('R'), 9);
        assert_eq!(translit('Z'), 9);
        assert_eq!(translit('0'), 0);
        assert_eq!(translit('9'), 9);
    }

    #[test]
    fn compute_matches_validate() {
        let v = "1HGCM82633A004352";
        assert_eq!(compute(v), Some('3'));
    }

    #[test]
    fn compute_rejects_wrong_length() {
        assert_eq!(compute("TOOSHORT"), None);
    }
}
