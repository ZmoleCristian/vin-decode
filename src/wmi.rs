use crate::Error;

const FORBIDDEN: &[char] = &['I', 'O', 'Q'];

pub fn validate_chars(s: &str) -> crate::Result<()> {
    if s.len() != 17 {
        return Err(Error::InvalidLength(s.len()));
    }
    for c in s.chars() {
        if !c.is_ascii_alphanumeric() {
            return Err(Error::InvalidChar(c));
        }
        if FORBIDDEN.contains(&c) {
            return Err(Error::ForbiddenChar(c));
        }
    }
    Ok(())
}

/// Map a VIN's first character (region code) to its ISO 3779 region name.
///
/// Returns `None` for unassigned codes (`0`, non-alphanumeric).
pub fn region(first: char) -> Option<&'static str> {
    match first {
        'A'..='H' => Some("Africa"),
        'J'..='R' => Some("Asia"),
        'S'..='Z' => Some("Europe"),
        '1'..='5' => Some("North America"),
        '6'..='7' => Some("Oceania"),
        '8'..='9' => Some("South America"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_clean_vin() {
        assert!(validate_chars("1HGCM82633A004352").is_ok());
    }

    #[test]
    fn validate_rejects_short_or_long() {
        assert!(matches!(
            validate_chars("TOOSHORT"),
            Err(Error::InvalidLength(_))
        ));
        assert!(matches!(
            validate_chars("WAYTOOLONG12345678"),
            Err(Error::InvalidLength(_))
        ));
    }

    #[test]
    fn validate_rejects_forbidden_iqo() {
        for forbidden in ['I', 'O', 'Q'] {
            let mut s = String::from("1HGCM82633A004352");
            unsafe { s.as_bytes_mut()[5] = forbidden as u8 };
            let res = validate_chars(&s);
            assert!(
                matches!(res, Err(Error::ForbiddenChar(c)) if c == forbidden),
                "expected ForbiddenChar({}), got {:?}",
                forbidden,
                res
            );
        }
    }

    #[test]
    fn validate_rejects_non_alnum() {
        let s = "1HGCM82633A00435!";
        assert!(matches!(validate_chars(s), Err(Error::InvalidChar(_))));
    }

    #[test]
    fn region_buckets_full_coverage() {
        assert_eq!(region('A'), Some("Africa"));
        assert_eq!(region('H'), Some("Africa"));
        assert_eq!(region('J'), Some("Asia"));
        assert_eq!(region('R'), Some("Asia"));
        assert_eq!(region('S'), Some("Europe"));
        assert_eq!(region('Z'), Some("Europe"));
        assert_eq!(region('1'), Some("North America"));
        assert_eq!(region('5'), Some("North America"));
        assert_eq!(region('6'), Some("Oceania"));
        assert_eq!(region('7'), Some("Oceania"));
        assert_eq!(region('8'), Some("South America"));
        assert_eq!(region('9'), Some("South America"));
        assert_eq!(region('0'), None);
        assert_eq!(region('!'), None);
    }
}
