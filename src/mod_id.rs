use anyhow::{
    Result,
    bail,
};

pub fn resolve_mod_ids<'a, I>(requested_mods: &[String], candidates: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut candidates: Vec<&String> = candidates.into_iter().collect();
    candidates.sort();

    let mut resolved = Vec::new();
    for requested in requested_mods {
        let matches: Vec<&String> = candidates
            .iter()
            .copied()
            .filter(|candidate| mod_id_matches(requested, candidate))
            .collect();

        match matches.as_slice() {
            [candidate] => resolved.push((*candidate).clone()),
            [] => bail!("Mod ID {} not found.", requested),
            matches => bail!(
                "Mod ID {} is ambiguous; matches: {}",
                requested,
                matches
                    .iter()
                    .map(|candidate| candidate.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    Ok(resolved)
}

fn mod_id_matches(requested: &str, candidate: &str) -> bool {
    if requested.chars().count() != candidate.chars().count() {
        return false;
    }

    requested
        .chars()
        .zip(candidate.chars())
        .all(|(requested_char, candidate_char)| char_matches(requested_char, candidate_char))
}

fn char_matches(requested: char, candidate: char) -> bool {
    if candidate.is_ascii_alphabetic() {
        return requested.eq_ignore_ascii_case(&candidate);
    }

    if candidate == ' ' || !candidate.is_ascii_alphanumeric() {
        return requested == candidate || requested == '-' || requested == '_';
    }

    requested == candidate
}

#[cfg(test)]
mod tests {
    use super::resolve_mod_ids;

    #[test]
    fn matches_letters_case_insensitively() {
        let candidates = ["FancyMod".to_string()];

        let resolved = resolve_mod_ids(&["fancymod".to_string()], candidates.iter()).unwrap();

        assert_eq!(resolved, ["FancyMod"]);
    }

    #[test]
    fn matches_spaces_and_special_characters_with_hyphen_or_underscore() {
        let candidates = ["A B+C".to_string()];

        let hyphenated = resolve_mod_ids(&["A-B-C".to_string()], candidates.iter()).unwrap();
        let underscored = resolve_mod_ids(&["A_B_C".to_string()], candidates.iter()).unwrap();

        assert_eq!(hyphenated, ["A B+C"]);
        assert_eq!(underscored, ["A B+C"]);
    }

    #[test]
    fn rejects_ambiguous_matches() {
        let candidates = ["A B".to_string(), "A-B".to_string()];

        let result = resolve_mod_ids(&["A_B".to_string()], candidates.iter());

        assert!(result.is_err());
    }
}
