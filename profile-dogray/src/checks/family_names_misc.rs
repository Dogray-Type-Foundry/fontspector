use std::collections::{BTreeMap, BTreeSet};

use super::font_sets;
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert, Metadata};
use serde_json::json;
use skrifa::{raw::TableProvider, string::StringId};
use write_fonts::from_obj::ToOwnedTable;

const FSTYPE_RESTRICTIONS: [(u16, &str); 5] = [
    (0x0002, "The font must not be modified, embedded or exchanged without the legal owner's permission (Restricted License embedding)."),
    (0x0004, "The font may be embedded and temporarily loaded on the remote system, but documents using it must not be editable (Preview & Print embedding)."),
    (0x0008, "The font may be embedded, but only installed temporarily on other systems (Editable embedding)."),
    (0x0100, "The font may not be subsetted before embedding."),
    (0x0200, "Only bitmaps in the font may be embedded; no outline data."),
];

#[check(
    id = "dogray/fstype",
    rationale = "
        OS/2.fsType is a legacy field that tells applications what embedding the
        font's licence allows. Any non-zero value restricts how users can embed
        the font in documents and apps; 0 means installable embedding with no
        restrictions. This warns when restrictions are set, so you can confirm
        they match the licence the font is sold under.
    ",
    proposal = "Dogray profile",
    title = "OS/2 fsType embedding restrictions",
    hotfix = fix_fstype,
)]
fn fstype(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let value = f.font().os2()?.fs_type();
    if value == 0 {
        return Ok(Status::just_one_pass());
    }
    let mut meaning: Vec<String> = FSTYPE_RESTRICTIONS
        .iter()
        .filter(|(mask, _)| value & mask != 0)
        .map(|(_, text)| text.to_string())
        .collect();
    if value & 0b1111_1100_1111_0001 != 0 {
        meaning.push("Reserved bits are set, which is invalid.".to_string());
    }
    let message = format!(
        "fsType is {value}, which means:\n\n{}\n\nCheck this matches the font's licence; 0 would allow unrestricted (installable) embedding.",
        meaning.iter().map(|m| format!("* {m}")).collect::<Vec<_>>().join("\n")
    );
    let mut status = Status::warn("embedding-restrictions", &message);
    status.add_metadata(Metadata::TableProblem {
        table_tag: "OS/2".to_string(),
        field_name: Some("fsType".to_string()),
        actual: Some(json!(value)),
        expected: Some(json!(0)),
        message,
    });
    Ok(Box::new(vec![status].into_iter()))
}

fn fix_fstype(
    t: &mut Testable,
    _replies: Option<MoreInfoReplies>,
) -> Result<FixResult, FontspectorError> {
    let f = testfont!(t);
    let mut os2: write_fonts::tables::os2::Os2 = f.font().os2()?.to_owned_table();
    os2.fs_type = 0;
    t.set(f.rebuild_with_new_table(&os2)?);
    Ok(FixResult::Fixed)
}

#[check(
    id = "dogray/valid_nameid25",
    rationale = "
        Because of a bug in (at least) Adobe InDesign, the Variations PostScript
        Name Prefix (name ID 25) of an italic variable font must differ from its
        upright counterpart. The usual convention is to end it with 'Italic'. It
        must not contain spaces.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/3024",
    title = "Name ID 25 ends with 'Italic' in italic variable fonts"
)]
fn valid_nameid25(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    skip!(
        !f.is_variable_font(),
        "not-variable",
        "Not a variable font."
    );
    let mut problems = vec![];
    if f.style().is_some_and(|s| s.contains("Italic")) {
        for name in f.get_name_entry_strings(StringId::new(25)) {
            if !name.ends_with("Italic") {
                problems.push(Status::fail(
                    "nameid25-missing-italic",
                    &format!("Name ID 25 is '{name}'; in an italic variable font it should end with 'Italic'."),
                ));
            }
            if name.contains(' ') {
                problems.push(Status::fail(
                    "nameid25-has-spaces",
                    &format!("Name ID 25 is '{name}'; it must not contain spaces."),
                ));
            }
        }
    }
    return_result(problems)
}

#[check(
    id = "dogray/equal_codepoint_coverage",
    rationale = "
        All fonts in a family should cover the same codepoints. Otherwise a
        character present in the Regular but missing from the Italic, for example,
        shows up as a fake (slanted) italic or as tofu when the user switches
        style.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/4180",
    title = "All fonts in the family cover the same codepoints",
    implementation = "all"
)]
fn equal_codepoint_coverage(c: &TestableCollection, context: &Context) -> CheckFnResult {
    let fonts = TTF.from_collection(c);
    skip!(
        fonts.len() < 2,
        "no-siblings",
        "Needs at least two fonts to compare."
    );
    let per_font: Vec<(String, BTreeSet<u32>)> = fonts
        .iter()
        .map(|f| {
            let name = f.filename.file_name().map_or_else(
                || f.filename.to_string_lossy().to_string(),
                |n| n.to_string_lossy().to_string(),
            );
            (name, f.codepoints(Some(context)).into_iter().collect())
        })
        .collect();
    let all: BTreeSet<u32> = per_font
        .iter()
        .flat_map(|(_, cps)| cps.iter().copied())
        .collect();
    // codepoint -> fonts lacking it, grouped by the exact set of fonts lacking it
    let mut groups: BTreeMap<Vec<String>, Vec<u32>> = BTreeMap::new();
    for cp in all {
        let lacking: Vec<String> = per_font
            .iter()
            .filter(|(_, cps)| !cps.contains(&cp))
            .map(|(name, _)| name.clone())
            .collect();
        if !lacking.is_empty() {
            groups.entry(lacking).or_default().push(cp);
        }
    }
    if groups.is_empty() {
        return Ok(Status::just_one_pass());
    }
    let all_names: Vec<String> = per_font.iter().map(|(name, _)| name.clone()).collect();
    let rows = groups.iter().map(|(lacking, cps)| {
        vec![
            font_sets::describe(lacking, &all_names),
            cps.len().to_string(),
            inline_list(
                context,
                cps.iter().map(|cp| {
                    char::from_u32(*cp)
                        .map_or(format!("U+{cp:04X}"), |c| format!("`{c}` U+{cp:04X}"))
                }),
            ),
        ]
    });
    Ok(Status::just_one_fail(
        "codepoint-coverage-differs",
        &format!(
            "Characters that only some fonts of the family have:\n\n{}",
            markdown_table(context, &["Missing from", "Count", "Characters"], rows)
        ),
    ))
}
