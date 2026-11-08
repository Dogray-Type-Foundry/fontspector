use std::collections::HashSet;

use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert, TestFont};
use harfrust::{script, ShaperData};
use skrifa::{
    instance::{LocationRef, Size},
    MetadataProvider,
};
use unicode_joining_type::{get_joining_type, JoiningType};

use super::shaping::{face, shape};

const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';
const TATWEEL: u32 = 0x0640;
const ARABIC_PUNCTUATION: [(u32, &str); 3] = [
    (0x060C, "ARABIC COMMA"),
    (0x061B, "ARABIC SEMICOLON"),
    (0x061F, "ARABIC QUESTION MARK"),
];
const MIRRORED_PUNCTUATION: [char; 12] =
    ['(', ')', '[', ']', '{', '}', '<', '>', '«', '»', '‹', '›'];
/// Extended Arabic-Indic digits four and seven, which Dogray draws differently for Urdu.
const URDU_DIGITS: [u32; 2] = [0x06F4, 0x06F7];
/// Letters that only Urdu (among Persian/Urdu) needs: tteh, ddal, rreh, noon ghunna,
/// heh doachashmee, heh goal, yeh barree.
const URDU_LETTERS: [u32; 7] = [0x0679, 0x0688, 0x0691, 0x06BA, 0x06BE, 0x06C1, 0x06D2];

fn is_arabic_block(cp: u32) -> bool {
    matches!(cp, 0x0600..=0x06FF | 0x0750..=0x077F | 0x0870..=0x089F | 0x08A0..=0x08FF)
}

/// Arabic-script letters in the font that join (dual- or right-joining), sorted.
fn joining_letters(codepoints: &HashSet<u32>) -> Vec<(char, JoiningType)> {
    let mut letters: Vec<(char, JoiningType)> = codepoints
        .iter()
        .copied()
        .filter(|cp| is_arabic_block(*cp))
        .filter_map(char::from_u32)
        .map(|c| (c, get_joining_type(c)))
        .filter(|(_, jt)| matches!(jt, JoiningType::DualJoining | JoiningType::RightJoining))
        .collect();
    letters.sort_by_key(|(c, _)| *c);
    letters
}

fn label(f: &TestFont, c: char) -> String {
    let name = f
        .font()
        .charmap()
        .map(c)
        .map(|gid| f.glyph_name_for_id_synthesise(gid))
        .unwrap_or_default();
    format!("U+{:04X} {c} ({name})", c as u32)
}

#[check(
    id = "dogray/arabic_joining",
    rationale = "
        Every Arabic letter that joins must change shape with its position in
        the word. Dual-joining letters (beh, seen, ain...) need distinct
        initial, medial and final forms; right-joining letters (alef, dal,
        reh, waw...) need a distinct final form. A letter that keeps its
        isolated shape in a joining context breaks the connection in running
        text.

        Each letter in the font is shaped on its own and between zero-width
        joiners (which force the joining contexts), and the resulting glyphs
        are compared. A form identical to the isolated one is a FAIL. Joining
        forms that are identical to each other (e.g. initial and medial) are
        a WARN, since they connect on different sides.
    ",
    proposal = "Dogray profile",
    title = "Arabic letters have all their joining forms"
)]
fn arabic_joining(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let letters = joining_letters(&f.codepoints(Some(context)));
    skip!(
        letters.is_empty(),
        "no-arabic",
        "The font has no joining Arabic letters."
    );
    let hb_face = face(&t.contents)?;
    let data = ShaperData::new(&hb_face);
    let shaper = data.shaper(&hb_face).build();
    // Whatever the joiners themselves shape to (their own glyph, or a hidden space).
    let joiner_glyphs: HashSet<u32> = shape(&shaper, &ZWJ.to_string(), script::ARABIC, None, &[])
        .into_iter()
        .collect();
    let form = |text: String| -> Vec<u32> {
        shape(&shaper, &text, script::ARABIC, None, &[])
            .into_iter()
            .filter(|g| !joiner_glyphs.contains(g))
            .collect()
    };

    let mut not_joining = vec![];
    let mut duplicates = vec![];
    for (c, jt) in letters {
        let isol = form(c.to_string());
        let mut forms = vec![("fina", form(format!("{ZWJ}{c}")))];
        if jt == JoiningType::DualJoining {
            forms.insert(0, ("init", form(format!("{c}{ZWJ}"))));
            forms.insert(1, ("medi", form(format!("{ZWJ}{c}{ZWJ}"))));
        }
        let same_as_isol: Vec<&str> = forms
            .iter()
            .filter(|(_, glyphs)| *glyphs == isol)
            .map(|(name, _)| *name)
            .collect();
        if !same_as_isol.is_empty() {
            not_joining.push(format!(
                "{}: {} identical to the isolated form",
                label(&f, c),
                same_as_isol.join(", ")
            ));
            continue;
        }
        let mut same_pairs = vec![];
        for (i, (a, ga)) in forms.iter().enumerate() {
            for (b, gb) in forms.iter().skip(i + 1) {
                if ga == gb {
                    same_pairs.push(format!("{a} = {b}"));
                }
            }
        }
        if !same_pairs.is_empty() {
            duplicates.push(format!("{}: {}", label(&f, c), same_pairs.join(", ")));
        }
    }

    let mut problems = vec![];
    if !not_joining.is_empty() {
        problems.push(Status::fail(
            "forms-not-joining",
            &format!(
                "These letters keep their isolated shape where they should join:\n\n{}",
                bullet_list(context, not_joining)
            ),
        ));
    }
    if !duplicates.is_empty() {
        problems.push(Status::warn(
            "duplicate-joining-forms",
            &format!(
                "These letters use the same glyphs for different joining forms:\n\n{}",
                bullet_list(context, duplicates)
            ),
        ));
    }
    return_result(problems)
}

#[check(
    id = "dogray/arabic_support",
    rationale = "
        Besides the letters themselves, Arabic text needs some supporting
        characters and behaviour:

        * Tatweel (U+0640), used for justification and to show letter forms.
        * Zero-width non-joiner and joiner (U+200C, U+200D), with no advance
          width; ZWNJ is part of normal Persian spelling.
        * Arabic comma, semicolon and question mark (U+060C, U+061B, U+061F).
        * Brackets, guillemets and < > must mirror in right-to-left text,
          either through their Unicode mirror pair or an 'rtlm' feature.
        * In fonts that support Urdu, the Extended Arabic-Indic digits four
          and seven (U+06F4, U+06F7) need different Persian and Urdu shapes,
          chosen with 'locl' for the FAR and URD language systems.
    ",
    proposal = "Dogray profile",
    title = "Arabic fonts have the supporting characters and behaviour Arabic text needs"
)]
fn arabic_support(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let codepoints = f.codepoints(Some(context));
    skip!(
        joining_letters(&codepoints).is_empty(),
        "no-arabic",
        "The font has no joining Arabic letters."
    );
    let mut problems = vec![];

    if !codepoints.contains(&TATWEEL) {
        problems.push(Status::warn(
            "missing-tatweel",
            "The font lacks ARABIC TATWEEL (U+0640).",
        ));
    }

    let metrics = f
        .font()
        .glyph_metrics(Size::unscaled(), LocationRef::default());
    for (c, name) in [(ZWNJ, "ZERO WIDTH NON-JOINER"), (ZWJ, "ZERO WIDTH JOINER")] {
        match f.font().charmap().map(c) {
            None => problems.push(Status::warn(
                "missing-joiner-controls",
                &format!("The font lacks {name} (U+{:04X}).", c as u32),
            )),
            Some(gid) => {
                let advance = metrics.advance_width(gid).unwrap_or_default();
                if advance != 0.0 {
                    problems.push(Status::warn(
                        "joiner-has-width",
                        &format!(
                            "{name} (U+{:04X}, glyph {}) has an advance width of {advance}; it should be 0.",
                            c as u32,
                            f.glyph_name_for_id_synthesise(gid)
                        ),
                    ));
                }
            }
        }
    }

    let missing_punct: Vec<String> = ARABIC_PUNCTUATION
        .iter()
        .filter(|(cp, _)| !codepoints.contains(cp))
        .map(|(cp, name)| format!("U+{cp:04X} {name}"))
        .collect();
    if !missing_punct.is_empty() {
        problems.push(Status::fail(
            "missing-arabic-punctuation",
            &format!(
                "The font lacks Arabic punctuation:\n\n{}",
                bullet_list(context, missing_punct)
            ),
        ));
    }

    let hb_face = face(&t.contents)?;
    let data = ShaperData::new(&hb_face);
    let shaper = data.shaper(&hb_face).build();

    let unmirrored: Vec<String> = MIRRORED_PUNCTUATION
        .iter()
        .filter(|c| codepoints.contains(&(**c as u32)))
        .filter(|c| {
            let text = c.to_string();
            shape(&shaper, &text, script::ARABIC, None, &[])
                == shape(&shaper, &text, script::LATIN, None, &[])
        })
        .map(|c| {
            let pair = unicode_bidi_mirroring::get_mirrored(*c)
                .map(|m| format!(" (its mirror U+{:04X} {m} is missing)", m as u32))
                .unwrap_or_default();
            format!("U+{:04X} {c}{pair}", *c as u32)
        })
        .collect();
    if !unmirrored.is_empty() {
        problems.push(Status::warn(
            "unmirrored-punctuation",
            &format!(
                "These characters don't mirror in right-to-left text. Add the mirror character or an 'rtlm' substitution:\n\n{}",
                bullet_list(context, unmirrored)
            ),
        ));
    }

    let has_urdu = URDU_LETTERS.iter().all(|cp| codepoints.contains(cp))
        && URDU_DIGITS.iter().all(|cp| codepoints.contains(cp));
    if has_urdu {
        let same: Vec<String> = URDU_DIGITS
            .iter()
            .filter_map(|cp| char::from_u32(*cp))
            .filter(|c| {
                let text = c.to_string();
                shape(&shaper, &text, script::ARABIC, Some("URD"), &[])
                    == shape(&shaper, &text, script::ARABIC, Some("FAR"), &[])
            })
            .map(|c| label(&f, c))
            .collect();
        if !same.is_empty() {
            problems.push(Status::warn(
                "no-urdu-digit-forms",
                &format!(
                    "The font supports Urdu, but these digits shape the same for Persian (FAR) and Urdu (URD). Urdu needs its own forms through 'locl':\n\n{}",
                    bullet_list(context, same)
                ),
            ));
        }
    }

    return_result(problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontspector_checkapi::{
        codetesting::{
            assert_messages_contain, assert_results_contain, assert_skip, run_check, test_able,
        },
        StatusCode,
    };

    #[test]
    fn joining_flags_letters_without_forms() {
        let result = run_check(
            arabic_joining,
            test_able("notosansarabic/NotoSansArabic-Regular.ttf"),
        );
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("forms-not-joining".to_string()),
        );
        // Straight waw has no final form in this font; beh is fine.
        assert_messages_contain(&result, "U+08B1");
        assert!(!result.iter().flat_map(|r| r.subresults.iter()).any(|s| s
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("U+0628")));
    }

    #[test]
    fn skips_latin_fonts() {
        assert_skip(&run_check(
            arabic_joining,
            test_able("montserrat/Montserrat-Regular.ttf"),
        ));
        assert_skip(&run_check(
            arabic_support,
            test_able("montserrat/Montserrat-Regular.ttf"),
        ));
    }
}
