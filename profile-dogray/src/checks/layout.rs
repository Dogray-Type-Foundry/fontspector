use std::collections::BTreeSet;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use skrifa::{
    raw::{
        tables::{gdef::GlyphClassDef, layout::ScriptList},
        ReadError, TableProvider,
    },
    MetadataProvider,
};

#[check(
    id = "dogray/expected_features",
    rationale = "
        Fonts may lack expected OpenType features, especially when their sources
        were converted from older formats. This warns when a font has no
        features at all, has nothing beyond 'kern', or has GSUB features but no
        'aalt' (which Adobe apps use to show the alternates for a glyph).

        Complements has_features, which checks against a configured list. From
        The Type Founders' font_lacks_expected_features.
    ",
    proposal = "Dogray profile",
    title = "Font has a reasonable set of OpenType features"
)]
fn expected_features(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let all: BTreeSet<String> = f
        .feature_records(false)
        .map(|(r, _)| r.feature_tag().to_string())
        .collect();
    let gsub: BTreeSet<String> = f
        .feature_records(true)
        .map(|(r, _)| r.feature_tag().to_string())
        .collect();
    let problem = if all.is_empty() {
        Some((
            "lacking-basic-features",
            "The font has no OpenType features. Is that intended?".to_string(),
        ))
    } else if all.iter().all(|tag| tag == "kern") {
        Some((
            "lacking-basic-features",
            "The font has no features beyond 'kern'. Is that intended?".to_string(),
        ))
    } else if !gsub.is_empty() && !gsub.contains("aalt") {
        Some((
            "lacking-aalt-feature",
            format!(
                "The font has GSUB features ({}) but no 'aalt'. Adobe apps use 'aalt' to show the alternates of a selected glyph; it should list the other substitution features.",
                gsub.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ))
    } else {
        None
    };
    Ok(match problem {
        Some((code, message)) => Status::just_one_warn(code, &message),
        None => Status::just_one_pass(),
    })
}

/// "script lang" pairs declared in a ScriptList.
pub(crate) fn languagesystems(list: Result<ScriptList, ReadError>, out: &mut BTreeSet<String>) {
    let Ok(list) = list else { return };
    for rec in list.script_records() {
        let script_tag = rec.script_tag().to_string().trim().to_string();
        let Ok(script) = rec.script(list.offset_data()) else {
            continue;
        };
        if script.default_lang_sys().is_some() {
            out.insert(format!("{script_tag} dflt"));
        }
        for lang in script.lang_sys_records() {
            out.insert(format!(
                "{script_tag} {}",
                lang.lang_sys_tag().to_string().trim()
            ));
        }
    }
}

#[check(
    id = "dogray/languagesystems",
    rationale = "
        Fonts should declare the default language systems so that their features
        apply everywhere: 'DFLT dflt', and 'latn dflt' for fonts with Latin
        letters. In compiled fonts these are the Script/LangSys records of GSUB
        and GPOS; a language system counts as declared if it appears in either.

        From The Type Founders' languagesystems. Their source-level requirement
        that every 'locl' language be declared is automatically true in a
        compiled font, so only the defaults are checked here.
    ",
    proposal = "Dogray profile",
    title = "Default language systems are declared"
)]
fn languagesystems_check(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let mut present = BTreeSet::new();
    if let Ok(gsub) = f.font().gsub() {
        languagesystems(gsub.script_list(), &mut present);
    }
    if let Ok(gpos) = f.font().gpos() {
        languagesystems(gpos.script_list(), &mut present);
    }
    if present.is_empty() {
        return Ok(Status::just_one_skip(
            "no-layout-tables",
            "The font has no GSUB/GPOS language systems.",
        ));
    }
    let codepoints = f.codepoints(Some(context));
    let has_latin = ('A'..='Z')
        .chain('a'..='z')
        .any(|c| codepoints.contains(&(c as u32)));
    let mut required = vec!["DFLT dflt"];
    if has_latin {
        required.push("latn dflt");
    }
    let missing: Vec<&str> = required
        .into_iter()
        .filter(|ls| !present.contains(*ls))
        .collect();
    if missing.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Status::just_one_fail(
            "missing-languagesystems",
            &format!(
                "Missing language system declarations (not found in GSUB or GPOS): {}. Declared: {}.",
                missing.iter().map(|m| format!("languagesystem {m};")).collect::<Vec<_>>().join(" "),
                present.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        ))
    }
}

#[check(
    id = "dogray/gdef_spacing_glyphs_not_marks",
    rationale = "
        Glyphs in the GDEF mark class (3) get zero advance in shapers that respect
        GDEF, such as HarfBuzz, so a Latin letter or digit wrongly classed as a
        mark overlaps the next glyph ('bac' collapses). This usually comes from
        base glyphs using underscore-prefixed anchor names ('_top') in the
        sources.

        The reverse case, marks that have a width, is opentype/GDEF_spacing_marks.
        From The Type Founders' gdef_spacing_glyphs_not_marks.
    ",
    proposal = "Dogray profile",
    title = "Latin letters and digits are not GDEF mark glyphs"
)]
fn gdef_spacing_glyphs_not_marks(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    if f.font().gdef().is_err() {
        return Ok(Status::just_one_skip(
            "no-gdef",
            "The font has no GDEF table.",
        ));
    }
    let charmap = f.font().charmap();
    let bad: Vec<String> = ('0'..='9')
        .chain('A'..='Z')
        .chain('a'..='z')
        .filter_map(|c| charmap.map(c).map(|gid| (c, gid)))
        .filter(|(_, gid)| f.gdef_class(*gid) == GlyphClassDef::Mark)
        .map(|(c, gid)| format!("'{c}' ({})", f.glyph_name_for_id_synthesise(gid)))
        .collect();
    if bad.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Status::just_one_fail(
            "spacing-glyphs-in-gdef-mark-class",
            &format!(
                "These Latin letters or digits are in the GDEF mark class, so they get zero advance and overlap in HarfBuzz and many preview tools: {}. Check the anchor names in the sources (base glyphs should not use underscore-prefixed anchors).",
                bad.join(", ")
            ),
        ))
    }
}
