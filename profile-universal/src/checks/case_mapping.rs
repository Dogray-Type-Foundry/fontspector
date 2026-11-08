use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert, Metadata};
use serde_json::json;
use skrifa::MetadataProvider;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};
use write_fonts::types::GlyphId;

fn swapcase(c: &char) -> Option<char> {
    if c.is_uppercase() {
        let mut lc = c.to_lowercase();
        if lc.len() == 1 {
            lc.next()
        } else {
            None
        }
    } else if c.is_lowercase() {
        let mut uc = c.to_uppercase();
        if uc.len() == 1 {
            uc.next()
        } else {
            None
        }
    } else {
        None
    }
}
const CASE_MAPPING_EXCEPTIONS: [u32; 22] = [
    0x0192, // ƒ - Latin Small Letter F with Hook
    0x00B5, // µ - Micro Sign
    0x03C0, // π - Greek Small Letter Pi
    0x2126, // Ω - Ohm Sign
    0x03BC, // μ - Greek Small Letter Mu
    0x03A9, // Ω - Greek Capital Letter Omega
    0x0394, // Δ - Greek Capital Letter Delta
    0x0251, // ɑ - Latin Small Letter Alpha
    0x0261, // ɡ - Latin Small Letter Script G
    0x00FF, // ÿ - Latin Small Letter Y with Diaeresis
    0x0250, // ɐ - Latin Small Letter Turned A
    0x025C, // ɜ - Latin Small Letter Reversed Open E
    0x0252, // ɒ - Latin Small Letter Turned Alpha
    0x0271, // ɱ - Latin Small Letter M with Hook
    0x0282, // ʂ - Latin Small Letter S with Hook
    0x029E, // ʞ - Latin Small Letter Turned K
    0x0287, // ʇ - Latin Small Letter Turned T
    0x0127, // ħ - Latin Small Letter H with Stroke
    0x0140, // ŀ - Latin Small Letter L with Middle Dot
    0x023F, // ȿ - Latin Small Letter S with Swash Tail
    0x0240, // ɀ - Latin Small Letter Z with Swash Tail
    0x026B, // ɫ - Latin Small Letter L with Middle Tilde
];

#[check(
    id = "case_mapping",
    rationale = "
        Ensure that no glyph lacks its corresponding upper or lower counterpart
        (but only when unicode supports case-mapping).
    ",
    proposal = "https://github.com/googlefonts/fontbakery/issues/3230",
    title = "Ensure the font supports case swapping for all its glyphs."
)]
fn case_mapping(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let codepoints = f.codepoints(Some(context));
    let mut sorted: Vec<u32> = codepoints.iter().copied().collect();
    sorted.sort_unstable();
    let name = |c: char| unicode_names2::name(c).map_or("Unknown".to_string(), |s| s.to_string());
    let mut rows = vec![];
    let mut metadata = vec![];
    for codepoint in sorted {
        if CASE_MAPPING_EXCEPTIONS.contains(&codepoint) {
            continue;
        }
        let Some(c) = char::from_u32(codepoint)
            .filter(|c| matches!(c.general_category_group(), GeneralCategoryGroup::Letter))
        else {
            continue;
        };
        let Some(swapped) = swapcase(&c) else {
            continue;
        };
        if codepoints.contains(&(swapped as u32)) {
            continue;
        }
        let glyph = f.font().charmap().map(codepoint).unwrap_or(GlyphId::new(0));
        let glyph_name = f.glyph_name_for_id_synthesise(glyph);
        rows.push(vec![
            format!("`{c}` U+{codepoint:04X} {}", name(c)),
            glyph_name.clone(),
            format!("`{swapped}` U+{:04X} {}", swapped as u32, name(swapped)),
        ]);
        let message = format!("Missing case-swapping counterpart for U+{codepoint:04X}");
        metadata.push(Metadata::GlyphProblem {
            glyph_name,
            glyph_id: glyph.to_u32(),
            userspace_location: None,
            position: None,
            actual: Some(json!(format!("U+{codepoint:04X}"))),
            expected: Some(json!(format!(
                "U+{:04X} (case counterpart)",
                swapped as u32
            ))),
            message,
        });
    }
    if rows.is_empty() {
        return Ok(Status::just_one_pass());
    }
    let mut status = Status::fail(
        "missing-case-counterparts",
        &format!(
            "Letters without their other-case counterpart ({}):\n\n{}",
            rows.len(),
            markdown_table(context, &["Has", "Glyph", "Missing"], rows)
        ),
    );
    for m in metadata {
        status.add_metadata(m);
    }
    Ok(Box::new(vec![status].into_iter()))
}

#[cfg(test)]
mod tests {
    use super::case_mapping;
    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    #[test]
    fn test_case_mapping_fail() {
        let testable = test_able("merriweather/Merriweather-Regular.ttf");
        let results = run_check(case_mapping, testable);
        assert_results_contain(
            &results,
            StatusCode::Fail,
            Some("missing-case-counterparts".to_string()),
        );
    }

    #[test]
    fn test_case_mapping_pass() {
        let testable = test_able("cabin/Cabin-Regular.ttf");
        let results = run_check(case_mapping, testable);
        assert_pass(&results);
    }
}
