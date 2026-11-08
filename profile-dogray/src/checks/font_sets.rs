//! Describes a subset of a family's fonts in words ("the 9 Italic fonts")
//! instead of listing every file, when the file names make that possible.

use std::collections::BTreeSet;

/// Splits a file name into words: "SetsGroteskXS-BoldItalic.ttf" ->
/// Sets, Grotesk, XS, Bold, Italic.
fn tokens(file: &str) -> BTreeSet<String> {
    let stem = file.rsplit('/').next().unwrap_or(file);
    let stem = stem.rsplit_once('.').map_or(stem, |(s, _)| s);
    let mut out = BTreeSet::new();
    for part in stem.split(['-', '_', ' ', '[', ']', ',']) {
        let chars: Vec<char> = part.chars().collect();
        let mut word = String::new();
        for (i, c) in chars.iter().enumerate() {
            let prev = i.checked_sub(1).and_then(|p| chars.get(p));
            let next = chars.get(i + 1);
            // New word at lower->Upper ("SetsGrotesk") and at the last capital of a
            // run followed by lowercase ("XSRegular" -> XS, Regular).
            let boundary = c.is_uppercase()
                && (prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit())
                    || (prev.is_some_and(|p| p.is_uppercase())
                        && next.is_some_and(|n| n.is_lowercase())));
            if boundary && !word.is_empty() {
                out.insert(std::mem::take(&mut word));
            }
            word.push(*c);
        }
        if !word.is_empty() {
            out.insert(word);
        }
    }
    out
}

fn base(file: &str) -> &str {
    file.rsplit('/').next().unwrap_or(file)
}

/// "all 37 fonts", "the 9 Italic fonts", "the 18 L and M fonts", or the file
/// names themselves when no naming pattern picks out exactly `subset`.
pub(crate) fn describe(subset: &[String], all: &[String]) -> String {
    let n = subset.len();
    if n == all.len() {
        return format!("all {n} fonts");
    }
    if let Some(words) = by_words(subset, all) {
        return words;
    }
    // Otherwise name what's left out, when that's simpler.
    let rest: Vec<String> = all
        .iter()
        .filter(|f| !subset.contains(f))
        .cloned()
        .collect();
    if let Some(words) = by_words(&rest, all) {
        return format!("every font except {words} ({n} fonts)");
    }
    if rest.len() <= 3 && n > rest.len() {
        let names: Vec<&str> = rest.iter().map(|f| base(f)).collect();
        return format!("every font except {} ({n} fonts)", names.join(", "));
    }
    let names: Vec<&str> = subset.iter().map(|f| base(f)).collect();
    if n == 1 {
        names.join("")
    } else {
        format!("{n} fonts: {}", names.join(", "))
    }
}

/// "the 9 Italic fonts" when up to three file-name words pick out exactly `subset`.
fn by_words(subset: &[String], all: &[String]) -> Option<String> {
    let n = subset.len();
    let others: Vec<&String> = all.iter().filter(|f| !subset.contains(f)).collect();
    let other_tokens: BTreeSet<String> = others.iter().flat_map(|f| tokens(f)).collect();
    // Words that only fonts in the subset have, most widespread first.
    let mut candidates: Vec<(String, BTreeSet<&String>)> = subset
        .iter()
        .flat_map(|f| tokens(f))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|t| !other_tokens.contains(t))
        .map(|t| {
            let having: BTreeSet<&String> =
                subset.iter().filter(|f| tokens(f).contains(&t)).collect();
            (t, having)
        })
        .collect();
    candidates.sort_by_key(|(_, having)| std::cmp::Reverse(having.len()));
    candidates.truncate(30);
    // The smallest set of up to three words whose fonts are exactly the subset.
    let covers = |pick: &[usize]| {
        let covered: BTreeSet<&String> = pick
            .iter()
            .filter_map(|i| candidates.get(*i))
            .flat_map(|(_, having)| having.iter().copied())
            .collect();
        covered.len() == n
    };
    let k = candidates.len();
    let mut chosen: Vec<String> = vec![];
    'search: for size in 1..=3usize.min(k) {
        let mut pick: Vec<usize> = (0..size).collect();
        loop {
            if covers(&pick) {
                chosen = pick
                    .iter()
                    .filter_map(|i| candidates.get(*i).map(|(t, _)| t.clone()))
                    .collect();
                break 'search;
            }
            // Next combination of `size` indices out of `k`.
            let Some(i) = (0..size)
                .rev()
                .find(|i| pick.get(*i).is_some_and(|p| *p < k - size + i))
            else {
                break;
            };
            if let Some(p) = pick.get_mut(i) {
                *p += 1;
            }
            let start = pick.get(i).copied().unwrap_or(0);
            for (offset, slot) in pick.iter_mut().skip(i + 1).enumerate() {
                *slot = start + offset + 1;
            }
        }
    }
    let covered_all = !chosen.is_empty();
    if n > 1 && covered_all {
        let words = match chosen.as_slice() {
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            [] => String::new(),
        };
        return Some(format!("the {n} {words} fonts"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn splits_file_names_into_words() {
        let t = tokens("/x/SetsGroteskXS-BoldItalic.ttf");
        assert_eq!(
            t.into_iter().collect::<Vec<_>>(),
            ["Bold", "Grotesk", "Italic", "Sets", "XS"]
        );
    }

    #[test]
    fn names_style_groups() {
        let all = fonts(&[
            "F-Regular.ttf",
            "F-Bold.ttf",
            "F-Italic.ttf",
            "F-BoldItalic.ttf",
        ]);
        let italics = fonts(&["F-Italic.ttf", "F-BoldItalic.ttf"]);
        assert_eq!(describe(&italics, &all), "the 2 Italic fonts");
        assert_eq!(describe(&all, &all), "all 4 fonts");
        let uprights = fonts(&["F-Regular.ttf", "F-Bold.ttf"]);
        assert_eq!(
            describe(&uprights, &all),
            "every font except the 2 Italic fonts (2 fonts)"
        );
        let odd = fonts(&["F-Regular.ttf", "F-BoldItalic.ttf"]);
        assert_eq!(
            describe(&odd, &all),
            "2 fonts: F-Regular.ttf, F-BoldItalic.ttf"
        );
    }

    #[test]
    fn names_optical_size_groups() {
        let all = fonts(&[
            "SetsGroteskXS-Thin.ttf",
            "SetsGroteskS-Thin.ttf",
            "SetsGroteskL-Thin.ttf",
            "SetsGroteskM-Thin.ttf",
            "SetsGroteskL-Bold.ttf",
            "SetsGroteskM-Bold.ttf",
        ]);
        let lm = fonts(&[
            "SetsGroteskL-Thin.ttf",
            "SetsGroteskM-Thin.ttf",
            "SetsGroteskL-Bold.ttf",
            "SetsGroteskM-Bold.ttf",
        ]);
        assert_eq!(describe(&lm, &all), "the 4 L and M fonts");
    }
}
