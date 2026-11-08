use std::collections::BTreeMap;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use itertools::Itertools;
use skrifa::{raw::TableProvider, GlyphId, MetadataProvider};

const COMMON_WIDTH_MATH_CHARS: [char; 314] = [
    '+', '<', '=', '>', '¬', '±', '×', '÷', '∈', '∉', '∋', '∌', '−', '∓', '∔', '∝', '∟', '∠', '∡',
    '∢', '∷', '∸', '∹', '∺', '∻', '∼', '∽', '∾', '∿', '≁', '≂', '≃', '≄', '≅', '≆', '≇', '≈', '≉',
    '≊', '≋', '≌', '≍', '≎', '≏', '≐', '≑', '≒', '≓', '≖', '≗', '≘', '≙', '≚', '≛', '≜', '≝', '≞',
    '≟', '≠', '≡', '≢', '≣', '≤', '≥', '≦', '≧', '≨', '≩', '≭', '≮', '≯', '≰', '≱', '≲', '≳', '≴',
    '≵', '≶', '≷', '≸', '≹', '≺', '≻', '≼', '≽', '≾', '≿', '⊀', '⊁', '⊂', '⊃', '⊄', '⊅', '⊆', '⊇',
    '⊈', '⊉', '⊊', '⊋', '⊏', '⊐', '⊑', '⊒', '⊢', '⊣', '⊤', '⊥', '⊨', '⊰', '⊱', '⊲', '⊳', '⊴', '⊵',
    '⊹', '⊾', '⋇', '⋍', '⋐', '⋑', '⋕', '⋖', '⋗', '⋚', '⋛', '⋜', '⋝', '⋞', '⋟', '⋠', '⋡', '⋢', '⋣',
    '⋤', '⋥', '⋦', '⋧', '⋨', '⋩', '⋳', '⋵', '⋶', '⋸', '⋹', '⋻', '⋽', '⟀', '⟃', '⟄', '⟓', '⟔', '⥶',
    '⥸', '⥹', '⥻', '⥾', '⥿', '⦓', '⦔', '⦕', '⦖', '⦛', '⦜', '⦝', '⦞', '⦟', '⦠', '⦡', '⦢', '⦣', '⦤',
    '⦥', '⦨', '⦩', '⦪', '⦫', '⧣', '⧤', '⧥', '⧺', '⧻', '⨢', '⨣', '⨤', '⨥', '⨦', '⨧', '⨨', '⨩', '⨪',
    '⨫', '⨬', '⨳', '⩦', '⩧', '⩨', '⩩', '⩪', '⩫', '⩬', '⩭', '⩮', '⩯', '⩰', '⩱', '⩲', '⩳', '⩷', '⩸',
    '⩹', '⩺', '⩻', '⩼', '⩽', '⩾', '⩿', '⪀', '⪁', '⪂', '⪃', '⪄', '⪅', '⪆', '⪇', '⪈', '⪉', '⪊', '⪋',
    '⪌', '⪍', '⪎', '⪏', '⪐', '⪑', '⪒', '⪓', '⪔', '⪕', '⪖', '⪗', '⪘', '⪙', '⪚', '⪛', '⪜', '⪝', '⪞',
    '⪟', '⪠', '⪡', '⪢', '⪦', '⪧', '⪨', '⪩', '⪪', '⪫', '⪬', '⪭', '⪮', '⪯', '⪰', '⪱', '⪲', '⪳', '⪴',
    '⪵', '⪶', '⪷', '⪸', '⪹', '⪺', '⪽', '⪾', '⪿', '⫀', '⫁', '⫂', '⫃', '⫄', '⫅', '⫆', '⫇', '⫈', '⫉',
    '⫊', '⫋', '⫌', '⫏', '⫐', '⫑', '⫒', '⫓', '⫔', '⫕', '⫖', '⫟', '⫠', '⫡', '⫢', '⫤', '⫦', '⫧', '⫨',
    '⫩', '⫪', '⫫', '⫳', '⫴', '⫵', '⫶', '⫹', '⫺', '〒',
];

#[check(
    id = "math_signs_width",
    rationale = r#"#,
        It is a common practice to have math signs sharing the same width
        (preferably the same width as tabular figures accross the entire font family).

        This probably comes from the will to avoid additional tabular math signs
        knowing that their design can easily share the same width.
    "#,
    proposal = "https://github.com/fonttools/fontbakery/issues/3832",
    title = "Check math signs have the same width."
)]
fn math_signs_width(f: &Testable, context: &Context) -> CheckFnResult {
    let font = testfont!(f);
    let charmap = font.font().charmap();
    let hmtx = font.font().hmtx()?;
    // width -> (char, glyph), in the order of COMMON_WIDTH_MATH_CHARS
    let mut widths: BTreeMap<u16, Vec<(char, GlyphId)>> = BTreeMap::new();
    for (c, gid) in COMMON_WIDTH_MATH_CHARS
        .iter()
        .flat_map(|c| charmap.map(*c as u32).map(|gid| (*c, gid)))
    {
        let width = hmtx.advance(gid).unwrap_or(0);
        widths.entry(width).or_default().push((c, gid));
    }
    if widths.len() <= 1 {
        return Ok(Status::just_one_pass());
    }
    let num_glyphs = widths.values().map(|g| g.len()).sum::<usize>();
    let Some((common, common_count)) = widths
        .iter()
        .max_by_key(|(_, glyphs)| glyphs.len())
        .map(|(width, glyphs)| (*width, glyphs.len()))
    else {
        return Ok(Status::just_one_pass());
    };
    let rows = widths.iter().map(|(width, glyphs)| {
        let (label, diff) = if *width == common {
            (format!("**{width}** (most common)"), "—".to_string())
        } else {
            (
                width.to_string(),
                format!("{:+}", i32::from(*width) - i32::from(common)),
            )
        };
        let names = glyphs
            .iter()
            .map(|(c, gid)| format!("`{c}` {}", font.glyph_name_for_id_synthesise(*gid)))
            .join(", ");
        vec![label, diff, glyphs.len().to_string(), names]
    });
    Ok(Status::just_one_warn(
        "width-outliers",
        &format!(
            "{} of the font's {num_glyphs} math signs are {common} units wide; {} {}.\n\n{}",
            common_count,
            num_glyphs - common_count,
            if num_glyphs - common_count == 1 {
                "differs"
            } else {
                "differ"
            },
            markdown_table(context, &["Width", "vs. common", "Glyphs", "Which"], rows)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    #[test]
    fn test_check_math_signs_width_pass() {
        let testable = test_able("stixtwomath/STIXTwoMath-Regular.ttf");
        let results = run_check(super::math_signs_width, testable);
        assert_pass(&results);
    }

    #[test]
    fn test_check_math_signs_width_fail() {
        let testable = test_able("montserrat/Montserrat-Regular.ttf");
        let results = run_check(super::math_signs_width, testable);
        assert_results_contain(
            &results,
            StatusCode::Warn,
            Some("width-outliers".to_string()),
        );
    }
}
