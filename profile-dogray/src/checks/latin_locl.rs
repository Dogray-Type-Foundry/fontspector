use std::collections::HashSet;

use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert};
use harfrust::{script, Feature, Shaper, ShaperData};

use super::shaping::{face, feature, shape};

const TURKIC: [&str; 5] = ["TRK", "AZE", "CRT", "KAZ", "TAT"];
const ROMANIAN: [&str; 2] = ["ROM", "MOL"];

/// Keeps only the characters of `text` the font maps.
fn present(text: &str, codepoints: &HashSet<u32>) -> String {
    text.chars()
        .filter(|c| codepoints.contains(&(*c as u32)))
        .collect()
}

/// The languages in `langs` for which `text` shapes exactly as it does by default.
fn unlocalized<'a>(
    shaper: &Shaper,
    text: &str,
    features: &[Feature],
    langs: &[&'a str],
) -> Vec<&'a str> {
    let default = shape(shaper, text, script::LATIN, None, features);
    langs
        .iter()
        .copied()
        .filter(|lang| shape(shaper, text, script::LATIN, Some(lang), features) == default)
        .collect()
}

#[check(
    id = "dogray/latin_locl",
    rationale = "
        Some Latin-script languages need localized forms through the 'locl'
        feature. When a font has the glyphs a language needs, its 'locl' rules
        should be in place too. Each case is shaped with and without the
        language system, and the check reports where nothing changes:

        * Turkish, Azerbaijani, Crimean Tatar, Kazakh, Tatar (TRK, AZE, CRT,
          KAZ, TAT): 'i' must keep its dot under 'smcp' (it becomes İ, not
          I), and the 'fi' ligature must not swallow the dot of i. Checked
          when the font has İ (U+0130) and the feature/ligature in question.
        * Romanian, Moldavian (ROM, MOL): Ş ş Ţ ţ (cedilla) should become the
          comma-below forms Ș ș Ț ț, which legacy Romanian text encodes with
          the cedilla characters. Checked when the font has both forms.
        * Catalan (CAT): l·l and L·L (punt volat) should use a spacing-adjusted
          middle dot. Checked when the font has ŀ (U+0140).
        * Dutch (NLD): í and Í followed by j/J should get an accented j (íj́,
          ÍJ́). Checked when the font has ĳ or Ĳ (U+0133, U+0132).
        * Polish (PLK): a 'kreska' version of the acute accent is a design
          choice, so its absence is only reported as INFO.
    ",
    proposal = "Dogray profile",
    title = "Latin languages get their localized forms"
)]
fn latin_locl(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let codepoints = f.codepoints(Some(context));
    skip!(
        !('a'..='z').all(|c| codepoints.contains(&(c as u32))),
        "no-latin",
        "The font has no basic Latin lowercase."
    );
    let hb_face = face(&t.contents)?;
    let data = ShaperData::new(&hb_face);
    let shaper = data.shaper(&hb_face).build();
    let mut problems = vec![];
    let mut tested = false;

    // Turkic dotted i
    if codepoints.contains(&0x0130) {
        let smcp: Vec<Feature> = feature("smcp").into_iter().collect();
        let smcp_changes_i = shape(&shaper, "i", script::LATIN, None, &smcp)
            != shape(&shaper, "i", script::LATIN, None, &[]);
        if smcp_changes_i {
            tested = true;
            let missing = unlocalized(&shaper, "i", &smcp, &TURKIC);
            if !missing.is_empty() {
                problems.push(Status::warn(
                    "turkic-smcp-i",
                    &format!(
                        "With 'smcp', 'i' shapes the same for {} as by default, so it loses its dot (it should become a small-cap İ).",
                        missing.join(", ")
                    ),
                ));
            }
        }
        if shape(&shaper, "fi", script::LATIN, None, &[]).len() == 1 {
            tested = true;
            let missing = unlocalized(&shaper, "fi", &[], &TURKIC);
            if !missing.is_empty() {
                problems.push(Status::warn(
                    "turkic-fi-ligature",
                    &format!(
                        "'fi' forms the default ligature for {}, which hides the dot of i (i and ı are different letters there).",
                        missing.join(", ")
                    ),
                ));
            }
        }
    }

    // Romanian comma below
    let has_commas = [0x0218, 0x0219, 0x021A, 0x021B]
        .iter()
        .any(|cp| codepoints.contains(cp));
    let cedillas = present("ŞşŢţ", &codepoints);
    if has_commas && !cedillas.is_empty() {
        tested = true;
        let missing = unlocalized(&shaper, &cedillas, &[], &ROMANIAN);
        if !missing.is_empty() {
            problems.push(Status::warn(
                "romanian-comma-below",
                &format!(
                    "'{cedillas}' shapes the same for {} as by default; Romanian should get the comma-below forms (Ș ș Ț ț).",
                    missing.join(", ")
                ),
            ));
        }
    }

    // Catalan punt volat
    let punt = present("l·l", &codepoints);
    if codepoints.contains(&0x0140) && punt.chars().count() == 3 {
        tested = true;
        let text = format!("{punt} {}", present("L·L", &codepoints));
        if !unlocalized(&shaper, &text, &[], &["CAT"]).is_empty() {
            problems.push(Status::warn(
                "catalan-punt-volat",
                "'l·l' / 'L·L' shape the same for Catalan (CAT) as by default; the middle dot should be adjusted for the punt volat.",
            ));
        }
    }

    // Dutch IJ acute
    if codepoints.contains(&0x0133) || codepoints.contains(&0x0132) {
        let text = [present("íj", &codepoints), present("ÍJ", &codepoints)]
            .into_iter()
            .filter(|s| s.chars().count() == 2)
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            tested = true;
            if !unlocalized(&shaper, &text, &[], &["NLD"]).is_empty() {
                problems.push(Status::warn(
                    "dutch-ij-acute",
                    &format!(
                        "'{text}' shapes the same for Dutch (NLD) as by default; the j/J after an accented i/I should get an acute too."
                    ),
                ));
            }
        }
    }

    // Polish kreska (design choice: INFO)
    let kreska = present("ĆćŃńÓóŚśŹź", &codepoints);
    if kreska.chars().count() == 10 {
        tested = true;
        if !unlocalized(&shaper, &kreska, &[], &["PLK"]).is_empty() {
            problems.push(Status::info(
                "no-polish-kreska",
                "The font has no Polish (PLK) 'kreska' forms of the acute. That's a design choice; this is only a note.",
            ));
        }
    }

    skip!(
        !tested,
        "nothing-to-test",
        "The font has none of the glyphs that call for localized Latin forms."
    );
    return_result(problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontspector_checkapi::{
        codetesting::{assert_results_contain, run_check, test_able},
        StatusCode,
    };

    #[test]
    fn flags_missing_turkic_locl() {
        let result = run_check(
            latin_locl,
            test_able("notosans/NotoSans-VariableFont_wdth,wght.ttf"),
        );
        assert_results_contain(&result, StatusCode::Warn, Some("turkic-smcp-i".to_string()));
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("turkic-fi-ligature".to_string()),
        );
    }

    #[test]
    fn flags_missing_romanian_and_catalan_locl() {
        let result = run_check(
            latin_locl,
            test_able("source-sans-pro/OTF/SourceSansPro-Regular.otf"),
        );
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("romanian-comma-below".to_string()),
        );
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("catalan-punt-volat".to_string()),
        );
    }

    #[test]
    fn montserrat_only_notes_kreska() {
        let result = run_check(latin_locl, test_able("montserrat/Montserrat-Regular.ttf"));
        assert_results_contain(
            &result,
            StatusCode::Info,
            Some("no-polish-kreska".to_string()),
        );
        assert!(result
            .iter()
            .flat_map(|r| r.subresults.iter())
            .all(|s| s.severity == StatusCode::Info));
    }
}
