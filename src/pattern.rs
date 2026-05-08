pub fn confidence(pattern: &str, vds: &str, vis: &str) -> f64 {
    let combined = format!("{vds}{vis}");
    let mut parts = pattern.split('|');
    let actual = match parts.next() {
        Some(p) => p,
        None => return 0.0,
    };
    let meta: Vec<&str> = parts.collect();

    if !meta.is_empty() && actual.chars().count() == 5 {
        return vis_score(actual, vis, meta[0]);
    }

    if actual.is_empty() || combined.is_empty() {
        return 0.0;
    }
    if !matches_simple(&combined, actual) {
        return 0.0;
    }
    score_simple(actual, &combined)
}

#[allow(dead_code)]
pub fn matches(pattern: &str, vds: &str, vis: &str) -> bool {
    let combined = format!("{vds}{vis}");
    let mut parts = pattern.split('|');
    let Some(actual) = parts.next() else {
        return false;
    };
    let meta: Vec<&str> = parts.collect();

    if !meta.is_empty() && actual.chars().count() == 5 {
        return vis_score(actual, vis, meta[0]) > 0.0;
    }

    matches_simple(&combined, actual)
}

/// Score the VIS-side metadata of a vPIC pattern (the part after `|`).
///
/// Meta is two slots: meta[0] = year code (VIS[0] = VIN pos 10), meta[1] =
/// plant code (VIS[1] = VIN pos 11). Each can be a literal char, `*`
/// wildcard, or a `[ABC]`/`[A-Z]` char class. Returns 0.0 on any mismatch
/// so wrong-plant rows can't outrank correct ones at weight=0.
fn vis_score(_actual: &str, vis: &str, meta: &str) -> f64 {
    let mc: Vec<char> = meta.chars().collect();
    let vc: Vec<char> = vis.chars().collect();
    let mut mi = 0;
    let mut vi = 0;
    let mut score: f64 = 0.0;
    let mut slots: f64 = 0.0;
    while mi < mc.len() && vi < vc.len() {
        let m = mc[mi];
        let v = vc[vi];
        if m == '[' {
            let close = match mc[mi + 1..].iter().position(|c| *c == ']') {
                Some(off) => mi + 1 + off,
                None => return 0.0,
            };
            if !in_class(v, &mc[mi + 1..close]) {
                return 0.0;
            }
            score += 0.9;
            slots += 1.0;
            mi = close + 1;
            vi += 1;
        } else if m == '*' {
            score += 0.5;
            slots += 1.0;
            mi += 1;
            vi += 1;
        } else {
            if m != v {
                return 0.0;
            }
            score += 1.0;
            slots += 1.0;
            mi += 1;
            vi += 1;
        }
    }
    if slots == 0.0 {
        return 0.0;
    }
    (score / slots).clamp(0.0, 1.0)
}

fn score_simple(pattern: &str, input: &str) -> f64 {
    let pcs: Vec<char> = pattern.chars().collect();
    let ics: Vec<char> = input.chars().collect();
    let mut exact = 0.0;
    let mut class = 0.0;
    let mut wild = 0.0;
    let mut total = 0.0;
    let mut pi = 0;
    let mut ii = 0;
    while pi < pcs.len() && ii < ics.len() {
        let p = pcs[pi];
        let i = ics[ii];
        if p == '[' {
            let close = match pcs[pi + 1..].iter().position(|c| *c == ']') {
                Some(off) => pi + 1 + off,
                None => break,
            };
            let content: String = pcs[pi + 1..close].iter().collect();
            class += if content.contains('-') { 0.7 } else { 0.8 };
            total += 1.0;
            pi = close + 1;
            ii += 1;
        } else if p == '*' {
            wild += 1.0;
            total += 1.0;
            pi += 1;
            ii += 1;
        } else {
            if p == i {
                exact += 1.0;
            }
            total += 1.0;
            pi += 1;
            ii += 1;
        }
    }
    if total == 0.0 {
        return 0.0;
    }
    let raw: f64 = (exact + class + wild * 0.5) / total;
    raw.clamp(0.0, 1.0)
}

pub fn matches_simple(input: &str, pattern: &str) -> bool {
    let pcs: Vec<char> = pattern.chars().collect();
    let ics: Vec<char> = input.chars().collect();
    let mut pi = 0;
    let mut ii = 0;
    while pi < pcs.len() && ii < ics.len() {
        let p = pcs[pi];
        let i = ics[ii];
        if p == '[' {
            let close = match pcs[pi + 1..].iter().position(|c| *c == ']') {
                Some(off) => pi + 1 + off,
                None => return false,
            };
            if !in_class(i, &pcs[pi + 1..close]) {
                return false;
            }
            pi = close + 1;
            ii += 1;
        } else if p == '*' {
            if pi == pcs.len() - 1 {
                return true;
            }
            pi += 1;
            ii += 1;
        } else {
            if p != i {
                return false;
            }
            pi += 1;
            ii += 1;
        }
    }
    pi >= pcs.len() || (pi == pcs.len() - 1 && pcs[pi] == '*')
}

fn in_class(c: char, content: &[char]) -> bool {
    let mut i = 0;
    while i < content.len() {
        if i + 2 < content.len() && content[i + 1] == '-' {
            if c >= content[i] && c <= content[i + 2] {
                return true;
            }
            i += 3;
        } else {
            if content[i] == c {
                return true;
            }
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_full_score() {
        let s = confidence("CM82*", "CM8263", "33A004352");
        assert!(s > 0.85, "got {}", s);
    }

    #[test]
    fn no_match_zero_score() {
        assert_eq!(confidence("XYZ12", "CM8263", "33A004352"), 0.0);
    }

    #[test]
    fn wildcard_only_half_score() {
        let s = confidence("*****", "CM8263", "33A004352");
        assert!((s - 0.5).abs() < 1e-9);
    }

    #[test]
    fn char_class_range_partial() {
        let s = confidence("[A-Z]****", "CM8263", "33A004352");
        assert!(s > 0.5);
    }

    #[test]
    fn char_class_explicit_higher_than_range() {
        let exp = confidence("[CM]****", "CM8263", "33A004352");
        let rng = confidence("[A-Z]****", "CM8263", "33A004352");
        assert!(exp > rng);
    }

    #[test]
    fn matches_simple_handles_classes() {
        assert!(matches_simple("CM8263", "C[A-Z]****"));
        assert!(!matches_simple("1M8263", "C[A-Z]****"));
    }

    #[test]
    fn matches_simple_handles_wildcard_terminal() {
        assert!(matches_simple("CM8263", "C*"));
        assert!(matches_simple("CM8263", "*"));
    }

    #[test]
    fn vis_metadata_plant_match() {
        // VIS[1]='T' in "ATCA73155" — meta `*T` matches plant 'T'.
        let s = confidence("*****|*T", "EF14H8", "ATCA73155");
        assert_eq!(s, 0.75);
    }

    #[test]
    fn vis_metadata_plant_mismatch() {
        // VIS[1]='T' — meta `*Z` rejects.
        let s = confidence("*****|*Z", "EF14H8", "ATCA73155");
        assert_eq!(s, 0.0);
    }

    #[test]
    fn vis_metadata_wildcard_plant() {
        // Both slots wildcard — half score per slot.
        let s = confidence("*****|**", "EF14H8", "ATCA73155");
        assert_eq!(s, 0.5);
    }

    #[test]
    fn vis_metadata_class_plant() {
        // VIS[1]='T' inside class [STU] — class match at 0.9.
        let s = confidence("*****|*[STU]", "EF14H8", "ATCA73155");
        assert!(s > 0.65, "got {s}");
    }

    #[test]
    fn matches_top_level_branches() {
        assert!(matches("CM82*", "CM8263", "33A004352"));
        assert!(!matches("ZZZZ*", "CM8263", "33A004352"));
        assert!(matches("*****|*T", "EF14H8", "ATCA73155"));
        assert!(!matches("*****|*Z", "EF14H8", "ATCA73155"));
    }

    #[test]
    fn in_class_range_and_explicit() {
        assert!(in_class('B', &['A', '-', 'C']));
        assert!(!in_class('D', &['A', '-', 'C']));
        assert!(in_class('Z', &['Z']));
        assert!(in_class('1', &['0', '-', '9']));
    }
}
