use crate::{Error, Vin};

pub fn decode(vin: &Vin, current_year: u32) -> crate::Result<u32> {
    let code = vin.year_code();
    let pos7 = vin.as_str().as_bytes()[6] as char;
    let base = year_for_code(code).ok_or(Error::UnreadableYear(code))?;
    let candidates = [base, base + 30];
    let pre2010 = pos7.is_ascii_digit();
    let pick = if pre2010 {
        candidates[0]
    } else {
        candidates[1]
    };
    if pick > current_year + 1 {
        Ok(candidates[0])
    } else {
        Ok(pick)
    }
}

pub fn year_for_code(c: char) -> Option<u32> {
    match c {
        'A' => Some(1980),
        'B' => Some(1981),
        'C' => Some(1982),
        'D' => Some(1983),
        'E' => Some(1984),
        'F' => Some(1985),
        'G' => Some(1986),
        'H' => Some(1987),
        'J' => Some(1988),
        'K' => Some(1989),
        'L' => Some(1990),
        'M' => Some(1991),
        'N' => Some(1992),
        'P' => Some(1993),
        'R' => Some(1994),
        'S' => Some(1995),
        'T' => Some(1996),
        'V' => Some(1997),
        'W' => Some(1998),
        'X' => Some(1999),
        'Y' => Some(2000),
        '1' => Some(2001),
        '2' => Some(2002),
        '3' => Some(2003),
        '4' => Some(2004),
        '5' => Some(2005),
        '6' => Some(2006),
        '7' => Some(2007),
        '8' => Some(2008),
        '9' => Some(2009),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vin_with(year_code: char, pos7: char) -> Vin {
        let mut s = String::from("1HGCM82633A004352");
        let bytes = unsafe { s.as_bytes_mut() };
        bytes[6] = pos7 as u8;
        bytes[9] = year_code as u8;
        Vin::new(s).unwrap()
    }

    #[test]
    fn pre2010_uses_first_block() {
        let v = vin_with('3', '6');
        assert_eq!(decode(&v, 2026).unwrap(), 2003);
    }

    #[test]
    fn post2010_uses_second_block() {
        let v = vin_with('K', 'A');
        assert_eq!(decode(&v, 2026).unwrap(), 2019);
    }

    #[test]
    fn post2010_block_used_when_pos7_letter() {
        let v = vin_with('F', 'F');
        assert_eq!(decode(&v, 2026).unwrap(), 2015);
    }

    #[test]
    fn year_clamps_to_first_block_when_future() {
        let v = vin_with('M', 'A');
        assert_eq!(decode(&v, 2019).unwrap(), 1991);
    }

    #[test]
    fn unreadable_year_code() {
        let v = vin_with('U', 'A');
        assert!(matches!(
            decode(&v, 2026),
            Err(crate::Error::UnreadableYear(_))
        ));
    }

    #[test]
    fn full_letter_table_pre2010() {
        for (code, year) in [
            ('A', 1980),
            ('B', 1981),
            ('Y', 2000),
            ('1', 2001),
            ('9', 2009),
        ] {
            assert_eq!(year_for_code(code), Some(year), "code {}", code);
        }
    }

    #[test]
    fn invalid_year_codes_rejected() {
        for c in ['I', 'O', 'Q', 'U', 'Z', '0', '\0'] {
            assert_eq!(year_for_code(c), None, "code {}", c);
        }
    }
}
