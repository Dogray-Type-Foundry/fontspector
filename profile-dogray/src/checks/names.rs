use std::collections::BTreeMap;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert, TestFont};
use skrifa::{
    raw::{
        tables::{
            head::MacStyle,
            os2::SelectionFlags,
            stat::{AxisValue, AxisValueTableFlags},
        },
        TableProvider,
    },
    string::StringId,
    MetadataProvider,
};

/// The English (or first) string for a name ID.
fn english(f: &TestFont, id: StringId) -> Option<String> {
    f.font()
        .localized_strings(id)
        .english_or_first()
        .map(|s| s.chars().collect())
}

fn typographic_family(f: &TestFont) -> Option<String> {
    english(f, StringId::TYPOGRAPHIC_FAMILY_NAME).or_else(|| english(f, StringId::FAMILY_NAME))
}

#[check(
    id = "dogray/postscript_name_hyphen",
    rationale = "
        Adobe Express (as of July 2026) rejects uploaded fonts whose PostScript
        name (nameID 6) has no hyphen, and other systems may assume one too.
        PostScript names should look like 'FamilyName-Bold', not 'FamilyNameBold'.

        From The Type Founders' postscript_name_hyphen.
    ",
    proposal = "Dogray profile",
    title = "PostScript name (nameID 6) contains a hyphen"
)]
fn postscript_name_hyphen(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let names: Vec<String> = f
        .get_name_entry_strings(StringId::POSTSCRIPT_NAME)
        .collect();
    if names.is_empty() {
        return Ok(Status::just_one_fail(
            "missing-postscript-name",
            "The font has no PostScript name (nameID 6).",
        ));
    }
    let bad: Vec<String> = names.into_iter().filter(|n| !n.contains('-')).collect();
    if bad.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Status::just_one_fail(
            "postscript-name-missing-hyphen",
            &format!(
                "PostScript name '{}' has no hyphen, so the font may fail to upload to Adobe Express.",
                bad.join("', '")
            ),
        ))
    }
}

const RIBBI_SUFFIXES: [&str; 5] = [
    " Regular",
    " Bold Italic",
    " BoldItalic",
    " Italic",
    " Bold",
];

fn strip_ribbi(name: &str) -> &str {
    RIBBI_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

const FULL_NAME_LIMIT: usize = 32;
const FAMILY_NAME_LIMIT: usize = 31;
const POSTSCRIPT_NAME_LIMIT: usize = 29;
const VF_NAME_LIMIT: usize = 31;

/// Style names Word builds for a VF: family + every combination of the
/// non-elidable STAT labels (one per axis), ignoring "Regular" and "Italic".
fn stat_style_names(f: &TestFont) -> Result<Option<Vec<String>>, FontspectorError> {
    let Ok(stat) = f.font().stat() else {
        return Ok(None);
    };
    let mut per_axis: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    if let Some(Ok(subtable)) = stat.offset_to_axis_values() {
        for value in subtable.axis_values().iter().flatten() {
            let (axis, flags, name_id) = match &value {
                AxisValue::Format1(v) => (v.axis_index(), v.flags(), v.value_name_id()),
                AxisValue::Format2(v) => (v.axis_index(), v.flags(), v.value_name_id()),
                AxisValue::Format3(v) => (v.axis_index(), v.flags(), v.value_name_id()),
                AxisValue::Format4(_) => continue,
            };
            if flags.contains(AxisValueTableFlags::ELIDABLE_AXIS_VALUE_NAME) {
                continue;
            }
            let Some(label) = english(f, name_id) else {
                continue;
            };
            if label == "Regular" || label == "Italic" {
                continue;
            }
            per_axis.entry(axis).or_default().push(label);
        }
    }
    let mut combinations = vec![String::new()];
    for labels in per_axis.values() {
        combinations = combinations
            .iter()
            .flat_map(|prefix| {
                labels.iter().map(move |label| {
                    if prefix.is_empty() {
                        label.clone()
                    } else {
                        format!("{prefix} {label}")
                    }
                })
            })
            .collect();
    }
    Ok(Some(combinations))
}

#[check(
    id = "dogray/family_and_style_max_length",
    rationale = "
        Long names cause problems in Microsoft Office on Windows: GDI's LOGFONT
        face name is limited to 32 characters including the terminating NULL,
        so longer names fall back to another font for some accented letters and
        scripts such as Thai, and get truncated in Word's font menu. Long
        PostScript names cause trouble with some printers and older apps.

        The Dogray limits (all reported as warnings):

        * Full name (nameID 4), without a trailing RIBBI style: 32 characters

        * Family name (nameID 1), without a trailing RIBBI style: 31 characters

        * PostScript name (nameID 6): 29 characters

        * Variable fonts: the typographic family name plus every combination of
          non-elidable STAT style labels (Word builds names this way; 'Regular'
          and 'Italic' don't count): 31 characters. Without a STAT table, the
          family name plus each fvar instance name: 31 characters.

        Combines the Universal check with The Type Founders' and Kilotype's
        versions.
    ",
    proposal = "Dogray profile",
    title = "Family, style, PostScript and variable-font names are not too long"
)]
fn family_and_style_max_length(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    if !f.has_table(b"name") {
        return Ok(Status::just_one_fail("lacks-table", "No name table."));
    }
    let mut problems = vec![];
    let mut too_long = |code: &str, what: &str, name: &str, limit: usize| {
        let len = name.chars().count();
        if len > limit {
            problems.push(Status::warn(
                code,
                &format!("{what} '{name}' has {len} characters; the limit is {limit}."),
            ));
        }
    };
    for name in f.get_name_entry_strings(StringId::FULL_NAME) {
        too_long(
            "nameid4-too-long",
            "Full name (nameID 4)",
            strip_ribbi(&name),
            FULL_NAME_LIMIT,
        );
    }
    for name in f.get_name_entry_strings(StringId::FAMILY_NAME) {
        too_long(
            "nameid1-too-long",
            "Family name (nameID 1)",
            strip_ribbi(&name),
            FAMILY_NAME_LIMIT,
        );
    }
    for name in f.get_name_entry_strings(StringId::POSTSCRIPT_NAME) {
        too_long(
            "nameid6-too-long",
            "PostScript name (nameID 6)",
            &name,
            POSTSCRIPT_NAME_LIMIT,
        );
    }

    if f.is_variable_font() {
        let family = typographic_family(&f).unwrap_or_default();
        let (code, source, styles) = match stat_style_names(&f)? {
            Some(styles) => (
                "familyname-plus-stat-entries-too-long",
                "STAT style labels",
                styles,
            ),
            None => (
                "fvar-instance-too-long",
                "fvar instance name",
                f.font()
                    .named_instances()
                    .iter()
                    .filter_map(|i| english(&f, i.subfamily_name_id()))
                    .collect(),
            ),
        };
        let long: Vec<String> = styles
            .iter()
            .map(|style| {
                if style.is_empty() {
                    family.clone()
                } else {
                    format!("{family} {style}")
                }
            })
            .filter(|name| name.chars().count() > VF_NAME_LIMIT)
            .map(|name| format!("'{name}' ({} characters)", name.chars().count()))
            .collect();
        if !long.is_empty() {
            problems.push(Status::warn(
                code,
                &format!(
                    "These variable-font names (family name + {source}) exceed {VF_NAME_LIMIT} characters:\n\n{}",
                    bullet_list(context, long)
                ),
            ));
        }
    }
    return_result(problems)
}

#[check(
    id = "dogray/duplicated_names",
    rationale = "
        Duplicated full names (nameID 4) or PostScript names (nameID 6) within a
        family can stop fonts from being listed in design apps, and break
        automatically generated CSS classes and @font-face rules.

        Port of typenetwork/family/duplicated_names (Windows English records);
        unlike the original, duplicated PostScript names are actually reported.
    ",
    proposal = "Dogray profile",
    title = "No duplicated full or PostScript names within the family",
    implementation = "all"
)]
fn duplicated_names(c: &TestableCollection, context: &Context) -> CheckFnResult {
    let fonts = TTF.from_collection(c);
    let mut seen: BTreeMap<(u16, String), Vec<String>> = BTreeMap::new();
    for f in &fonts {
        let name = f.font().name()?;
        for rec in name.name_record() {
            let id = rec.name_id().to_u16();
            if rec.platform_id() != 3
                || rec.encoding_id() != 1
                || rec.language_id() != 0x409
                || !(id == 4 || id == 6)
            {
                continue;
            }
            let Ok(s) = rec.string(name.string_data()) else {
                continue;
            };
            seen.entry((id, s.chars().collect()))
                .or_default()
                .push(f.filename.to_string_lossy().to_string());
        }
    }
    let mut problems = vec![];
    for (id, code, label) in [
        (4, "duplicate-full-names", "full names"),
        (6, "duplicate-postscript-names", "PostScript names"),
    ] {
        let dupes: Vec<String> = seen
            .iter()
            .filter(|((nid, _), files)| *nid == id && files.len() > 1)
            .map(|((_, value), files)| format!("'{value}' in {}", files.join(", ")))
            .collect();
        if !dupes.is_empty() {
            problems.push(Status::fail(
                code,
                &format!(
                    "These {label} are duplicated:\n\n{}",
                    bullet_list(context, dupes)
                ),
            ));
        }
    }
    return_result(problems)
}

#[check(
    id = "dogray/style_linking",
    rationale = "
        Bold and italic fonts must be flagged consistently so that applications
        link them into style groups: OS/2 fsSelection, head macStyle, name ID 2
        and, for italics, a non-zero post.italicAngle.

        The font's style is taken from its filename (or, failing that, its
        flags). Port of fontwerk/style_linking.
    ",
    proposal = "Dogray profile",
    title = "Style linking entries are consistent"
)]
fn style_linking(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let style = f.style().unwrap_or("Regular").to_string();
    let is_bold = style == "Bold" || style == "BoldItalic";
    let is_italic = style.contains("Italic");
    let fs = f.font().os2()?.fs_selection();
    let mac = f.font().head()?.mac_style();
    let subfamily = english(&f, StringId::SUBFAMILY_NAME).unwrap_or_default();
    let mut errs = vec![];
    if is_bold {
        if !fs.contains(SelectionFlags::BOLD) {
            errs.push("OS/2 fsSelection should (most likely) have the Bold bit set.".to_string());
        }
        if !mac.contains(MacStyle::BOLD) {
            errs.push("head macStyle should (most likely) have the Bold bit set.".to_string());
        }
        if subfamily != "Bold" && subfamily != "Bold Italic" {
            let should = if is_italic { "Bold Italic" } else { "Bold" };
            errs.push(format!(
                "Name ID 2 is '{subfamily}' and should (most likely) be '{should}'."
            ));
        }
    }
    if is_italic {
        if f.font()
            .post()
            .is_ok_and(|p| p.italic_angle().to_f64() == 0.0)
        {
            errs.push("post.italicAngle should (most likely) be non-zero.".to_string());
        }
        if !fs.contains(SelectionFlags::ITALIC) {
            errs.push("OS/2 fsSelection should (most likely) have the Italic bit set.".to_string());
        }
        if !mac.contains(MacStyle::ITALIC) {
            errs.push("head macStyle should (most likely) have the Italic bit set.".to_string());
        }
        if subfamily != "Italic" && subfamily != "Bold Italic" {
            let should = if is_bold { "Bold Italic" } else { "Italic" };
            errs.push(format!(
                "Name ID 2 is '{subfamily}' and should (most likely) be '{should}'."
            ));
        }
    }
    return_result(
        errs.iter()
            .map(|e| Status::fail("style-linking-issue", e))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::strip_ribbi;

    #[test]
    fn strips_ribbi_suffixes() {
        assert_eq!(strip_ribbi("Brand Sans Bold Italic"), "Brand Sans");
        assert_eq!(strip_ribbi("Brand Sans Regular"), "Brand Sans");
        assert_eq!(strip_ribbi("Brand Sans Medium"), "Brand Sans Medium");
    }
}
