use std::collections::{BTreeMap, BTreeSet};

use fontspector_checkapi::{prelude::*, skip, FileTypeConvert, TestFont};
use skrifa::raw::{tables::layout::FeatureParams, TableProvider};

use super::{font_sets, layout::languagesystems};

/// Only exists in variable fonts (feature variations), so never compared.
const VARIABLE_ONLY_FEATURES: [&str; 1] = ["rvrn"];

struct Layout {
    name: String,
    /// "GSUB smcp", "GPOS kern", ...
    features: BTreeSet<String>,
    /// "latn TRK", ...
    languagesystems: BTreeSet<String>,
    /// ss01 -> "Single-storey a", cv01 -> ...
    feature_names: BTreeMap<String, String>,
}

fn layout(f: &TestFont) -> Layout {
    let name = f.filename.file_name().map_or_else(
        || f.filename.to_string_lossy().to_string(),
        |n| n.to_string_lossy().to_string(),
    );
    let mut features = BTreeSet::new();
    let mut feature_names = BTreeMap::new();
    let gsub_count = f
        .font()
        .gsub()
        .ok()
        .and_then(|t| t.feature_list().ok())
        .map_or(0, |l| l.feature_records().len());
    for (i, (record, feature)) in f.feature_records(false).enumerate() {
        let tag = record.feature_tag().to_string();
        if VARIABLE_ONLY_FEATURES.contains(&tag.as_str()) {
            continue;
        }
        let table = if i < gsub_count { "GSUB" } else { "GPOS" };
        features.insert(format!("{table} {tag}"));
        let name_id = match feature.ok().and_then(|f| f.feature_params()) {
            Some(Ok(FeatureParams::StylisticSet(p))) => Some(p.ui_name_id()),
            Some(Ok(FeatureParams::CharacterVariant(p))) => Some(p.feat_ui_label_name_id()),
            _ => None,
        };
        if let Some(label) = name_id.and_then(|id| f.get_name_entry_strings(id).next()) {
            feature_names.insert(tag, label);
        }
    }
    let mut langsys = BTreeSet::new();
    if let Ok(gsub) = f.font().gsub() {
        languagesystems(gsub.script_list(), &mut langsys);
    }
    if let Ok(gpos) = f.font().gpos() {
        languagesystems(gpos.script_list(), &mut langsys);
    }
    Layout {
        name,
        features,
        languagesystems: langsys,
        feature_names,
    }
}

/// Groups the items some fonts lack by the exact set of fonts lacking them:
/// one table row per group, the items in backticks.
fn missing_groups<'a>(
    fonts: &'a [Layout],
    items: impl Fn(&'a Layout) -> &'a BTreeSet<String>,
    shown: impl Fn(&str) -> String,
) -> Vec<Vec<String>> {
    let all: Vec<String> = fonts.iter().map(|l| l.name.clone()).collect();
    let every: BTreeSet<&String> = fonts.iter().flat_map(|l| items(l).iter()).collect();
    let mut groups: BTreeMap<Vec<&str>, Vec<&str>> = BTreeMap::new();
    for item in every {
        let lacking: Vec<&str> = fonts
            .iter()
            .filter(|l| !items(l).contains(item))
            .map(|l| l.name.as_str())
            .collect();
        if !lacking.is_empty() {
            groups.entry(lacking).or_default().push(item.as_str());
        }
    }
    groups
        .into_iter()
        .map(|(lacking, items)| {
            let lacking: Vec<String> = lacking.into_iter().map(String::from).collect();
            let mut shown_items: Vec<String> = items.iter().map(|i| shown(i)).collect();
            shown_items.dedup();
            vec![shown_items.join(", "), font_sets::describe(&lacking, &all)]
        })
        .collect()
}

#[check(
    id = "dogray/feature_parity",
    rationale = "
        All fonts of a family should offer the same OpenType features, the same
        language systems and the same names for their stylistic sets and
        character variants. Otherwise a feature or a language's 'locl'
        silently stops working when the user switches weight or optical size,
        and the feature menus of design apps change from style to style.

        Some features legitimately exist only in some styles (e.g. stylistic
        sets drawn for a single optical size), so missing features are only
        reported as INFO. Mismatched language systems and feature names are
        a WARN. 'rvrn' is ignored, since only variable fonts have it.
    ",
    proposal = "Dogray profile",
    title = "All fonts in the family have the same features, language systems and feature names",
    implementation = "all"
)]
fn feature_parity(c: &TestableCollection, context: &Context) -> CheckFnResult {
    let fonts: Vec<Layout> = TTF.from_collection(c).iter().map(layout).collect();
    skip!(
        fonts.len() < 2,
        "no-siblings",
        "Needs at least two fonts to compare."
    );
    let mut problems = vec![];

    // "GSUB smcp" -> `smcp`
    let feature_tag = |s: &str| format!("`{}`", s.split_once(' ').map_or(s, |(_, t)| t));
    let features = missing_groups(&fonts, |l| &l.features, feature_tag);
    if !features.is_empty() {
        problems.push(Status::info(
            "feature-mismatch",
            &format!(
                "Features that some fonts don't have:\n\n{}",
                markdown_table(context, &["Features", "Missing from"], features)
            ),
        ));
    }

    let langsys = missing_groups(&fonts, |l| &l.languagesystems, |s| format!("`{s}`"));
    if !langsys.is_empty() {
        problems.push(Status::warn(
            "languagesystem-mismatch",
            &format!(
                "Language systems (script, language) that some fonts don't declare:\n\n{}",
                markdown_table(context, &["Language systems", "Missing from"], langsys)
            ),
        ));
    }

    // tag -> name -> fonts using that name
    let mut names: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
    for l in &fonts {
        for (tag, label) in &l.feature_names {
            names
                .entry(tag)
                .or_default()
                .entry(label)
                .or_default()
                .push(&l.name);
        }
    }
    let all: Vec<String> = fonts.iter().map(|l| l.name.clone()).collect();
    let differing: Vec<Vec<String>> = names
        .into_iter()
        .filter(|(_, labels)| labels.len() > 1)
        .flat_map(|(tag, labels)| {
            let all = &all;
            labels.into_iter().map(move |(label, fonts)| {
                let fonts: Vec<String> = fonts.into_iter().map(String::from).collect();
                vec![
                    format!("`{tag}`"),
                    format!("“{label}”"),
                    font_sets::describe(&fonts, all),
                ]
            })
        })
        .collect();
    if !differing.is_empty() {
        problems.push(Status::warn(
            "feature-name-mismatch",
            &format!(
                "Stylistic set / character variant names that differ between fonts:\n\n{}",
                markdown_table(context, &["Feature", "Name", "Used in"], differing)
            ),
        ));
    }
    return_result(problems)
}
