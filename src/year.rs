//! VIN model-year code mapping (raw lookup only).
//!
//! The decoder no longer auto-resolves a model year — `Vehicle.model_year`
//! is always `None`. SAE-J853 maps each year code to TWO candidate years
//! 30 years apart, and brands disagree on which VIN position carries the
//! year. Returning a guessed year was wrong more often than helpful on
//! real corpora.
//!
//! Consumers who want the raw candidates can call [`Vin::year_candidates`]
//! or this module's [`year_for_code`] directly.

/// Map a VIN year code character to its first SAE-J853 cycle year (1980-2009).
/// Add 30 to get the second cycle (2010-2039). Returns `None` for invalid
/// codes (`I`/`O`/`Q`/`U`/`Z`/`0`).
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
