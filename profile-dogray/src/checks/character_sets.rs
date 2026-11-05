use std::collections::HashSet;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};

use super::charset_data::{
    DOGRAY_LATIN, KOEBERLIN_LATIN_L, KOEBERLIN_LATIN_M, KOEBERLIN_LATIN_S, KOEBERLIN_LATIN_XL,
    MAC_ROMAN, WINDOWS_1252,
};

/// Characters listed when a set is incomplete (all of them with --full-lists).
const SHOWN_MISSING: usize = 30;

struct SetReport {
    name: &'static str,
    total: usize,
    missing: Vec<u32>,
}

fn report(
    name: &'static str,
    set: impl IntoIterator<Item = u32>,
    have: &HashSet<u32>,
) -> SetReport {
    let set: HashSet<u32> = set.into_iter().collect();
    let mut missing: Vec<u32> = set.difference(have).copied().collect();
    missing.sort_unstable();
    SetReport {
        name,
        total: set.len(),
        missing,
    }
}

fn format_char(cp: u32) -> String {
    match char::from_u32(cp) {
        // Show combining marks on a dotted circle so they are visible
        Some(c)
            if [
                0x0300..=0x036F,
                0x0610..=0x061A,
                0x064B..=0x065F,
                0x0670..=0x0670,
                0x06D6..=0x06ED,
            ]
            .iter()
            .any(|r| r.contains(&cp)) =>
        {
            format!("U+{cp:04X} ◌{c}")
        }
        Some(c) => format!("U+{cp:04X} {c}"),
        None => format!("U+{cp:04X}"),
    }
}

fn cumulative(levels: &[&[u32]]) -> Vec<u32> {
    levels.iter().flat_map(|l| l.iter().copied()).collect()
}

#[check(
    id = "dogray/character_sets",
    rationale = "
        Reports how the font's character coverage compares with common
        character sets, so it is clear which sets the font supports fully and
        what is missing from the others. This is informational only.

        * Dogray Latin, the house Latin set (from the 'Dogray Latin' Glyphs
          filter; its suffixed glyphs such as .case and .loclCAT variants have
          no codepoint and are not checked).

        * Koeberlin Latin S, M, L and XL
          (https://github.com/koeberlin/Latin-Character-Sets). The levels are
          cumulative: Latin M includes Latin S, and so on.

        * Windows-1252 and Mac OS Roman (the printable characters of each code
          page; the Apple logo U+F8FF is not required).

        * For fonts containing Arabic: Google Fonts' GF Arabic Core and
          GF Arabic Plus glyph sets (https://github.com/googlefonts/glyphsets).

        Latin sets are evaluated for fonts with Latin letters, and Arabic sets
        for fonts with Arabic letters.

        Based on The Type Founders' minimum_character_coverage, which required a
        Western European set.
    ",
    proposal = "Dogray profile",
    title = "Character set coverage: Dogray Latin, Koeberlin Latin, Windows-1252, Mac Roman and Arabic"
)]
fn character_sets(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let have = f.codepoints(Some(context));

    let has_latin = have
        .iter()
        .any(|cp| (0x41..=0x5A).contains(cp) || (0x61..=0x7A).contains(cp));
    let mut reports = vec![
        report("Dogray Latin", DOGRAY_LATIN.iter().copied(), &have),
        report("Koeberlin Latin S", cumulative(&[KOEBERLIN_LATIN_S]), &have),
        report(
            "Koeberlin Latin M",
            cumulative(&[KOEBERLIN_LATIN_S, KOEBERLIN_LATIN_M]),
            &have,
        ),
        report(
            "Koeberlin Latin L",
            cumulative(&[KOEBERLIN_LATIN_S, KOEBERLIN_LATIN_M, KOEBERLIN_LATIN_L]),
            &have,
        ),
        report(
            "Koeberlin Latin XL",
            cumulative(&[
                KOEBERLIN_LATIN_S,
                KOEBERLIN_LATIN_M,
                KOEBERLIN_LATIN_L,
                KOEBERLIN_LATIN_XL,
            ]),
            &have,
        ),
        report("Windows-1252", WINDOWS_1252.iter().copied(), &have),
        report("Mac OS Roman", MAC_ROMAN.iter().copied(), &have),
    ];
    if !has_latin {
        reports.clear();
    }
    let has_arabic = have.iter().any(|cp| (0x0620..=0x064A).contains(cp));
    if has_arabic {
        for (name, id) in [
            ("GF Arabic Core", "GF_Arabic_Core"),
            ("GF Arabic Plus", "GF_Arabic_Plus"),
        ] {
            if let Some(coverage) = google_fonts_glyphsets::get_coverage(&have, id) {
                reports.push(report(
                    name,
                    coverage.has.into_iter().chain(coverage.missing),
                    &have,
                ));
            }
        }
    }

    let mut table = String::from("| Set | Coverage | Status |\n|---|---|---|\n");
    for r in &reports {
        let covered = r.total - r.missing.len();
        let pct = if r.total == 0 {
            0.0
        } else {
            covered as f64 * 100.0 / r.total as f64
        };
        let status = if r.missing.is_empty() {
            "✅ complete".to_string()
        } else {
            format!("{} missing", r.missing.len())
        };
        table.push_str(&format!(
            "| {} | {covered}/{} ({pct:.0}%) | {status} |\n",
            r.name, r.total
        ));
    }
    let complete: Vec<&str> = reports
        .iter()
        .filter(|r| r.missing.is_empty())
        .map(|r| r.name)
        .collect();
    let summary = if complete.is_empty() {
        "The font fully supports none of these sets.".to_string()
    } else {
        format!("Fully supported: {}.", complete.join(", "))
    };

    let mut details = String::new();
    // Only list what's missing from the first incomplete Koeberlin level and the code pages,
    // so the report stays readable.
    let first_incomplete_latin = reports
        .iter()
        .filter(|r| r.name.starts_with("Koeberlin"))
        .find(|r| !r.missing.is_empty())
        .map(|r| r.name);
    for r in reports.iter().filter(|r| !r.missing.is_empty()) {
        if r.name.starts_with("Koeberlin") && Some(r.name) != first_incomplete_latin {
            continue;
        }
        let shown: Vec<String> = if context.full_lists {
            r.missing.iter().map(|cp| format_char(*cp)).collect()
        } else {
            r.missing
                .iter()
                .take(SHOWN_MISSING)
                .map(|cp| format_char(*cp))
                .collect()
        };
        let more = r.missing.len().saturating_sub(shown.len());
        details.push_str(&format!(
            "\n\n**Missing from {}:** {}{}",
            r.name,
            shown.join(", "),
            if more > 0 {
                format!(" … and {more} more (use full lists to see all)")
            } else {
                String::new()
            }
        ));
    }
    if !has_latin {
        details.push_str(
            "\n\nNo Latin letters found, so the Latin sets and code pages were not evaluated.",
        );
    }
    if !has_arabic {
        details.push_str("\n\nNo Arabic letters found, so the Arabic sets were not evaluated.");
    }

    Ok(Status::just_one_info(
        "character-sets",
        &format!("{summary}\n\n{table}{details}"),
    ))
}
