use std::collections::BTreeMap;

use fontspector_checkapi::{prelude::*, FileTypeConvert};
use skrifa::{
    raw::{types::NameId, TableProvider},
    MetadataProvider,
};

/// Trailing words that name a width or optical-size subfamily rather than a
/// different family ("Brand Sans Condensed", "Brand Text").
const SUBFAMILY_WORDS: &[&str] = &[
    // widths
    "condensed",
    "cond",
    "cn",
    "compressed",
    "compact",
    "narrow",
    "extended",
    "ext",
    "expanded",
    "wide",
    "semicondensed",
    "extracondensed",
    "ultracondensed",
    "semiexpanded",
    "extraexpanded",
    "ultraexpanded",
    "semi",
    "extra",
    "ultra",
    // optical sizes
    "display",
    "text",
    "caption",
    "micro",
    "deck",
    "banner",
    "poster",
    "headline",
    "subhead",
    "small",
    "fine",
    "title",
    "big",
    // size letters used for optical sizes (Sets Grotesk XS/S/M/L)
    "xxxs",
    "xxs",
    "xs",
    "s",
    "m",
    "l",
    "xl",
    "xxl",
    "xxxl",
    // variable-font suffixes
    "vf",
    "var",
    "variable",
];

/// Family name with any trailing width/optical-size words removed.
fn superfamily(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1
        && words
            .last()
            .is_some_and(|w| SUBFAMILY_WORDS.contains(&w.to_lowercase().as_str()))
    {
        words.pop();
    }
    words.join(" ")
}

#[check(
    id = "dogray/consistent_family_name",
    rationale = "
        Like opentype/family/consistent_family_name, this compares the Typographic
        Family name (nameID 16, or nameID 1 if absent) across the fonts checked.

        Dogray families may be split into width or optical-size subfamilies, such as
        'Brand Sans' and 'Brand Sans Condensed' or 'Brand Text'. These are
        intentionally separate typographic families within one superfamily, so
        trailing width and optical-size words (including size letters such as
        XS, S, M, L) and variable-font suffixes (VF, Var) are ignored when
        comparing. Splitting a large family this way keeps each one's style menu
        manageable. The subfamilies found are reported for information.

        Replaces both opentype/family/consistent_family_name and
        typographic_family_name, which compare the same names.
    ",
    proposal = "Dogray profile",
    title = "Family names are consistent, allowing width and optical-size subfamilies",
    implementation = "all"
)]
fn consistent_family_name(c: &TestableCollection, _context: &Context) -> CheckFnResult {
    let fonts = TTF.from_collection(c);
    // superfamily -> family name -> files
    let mut families: BTreeMap<String, BTreeMap<String, Vec<(u16, String)>>> = BTreeMap::new();
    for font in fonts {
        let name = font
            .font()
            .localized_strings(NameId::TYPOGRAPHIC_FAMILY_NAME)
            .english_or_first()
            .or_else(|| {
                font.font()
                    .localized_strings(NameId::FAMILY_NAME)
                    .english_or_first()
            })
            .ok_or_else(|| {
                FontspectorError::General(format!(
                    "Font {} is missing a Family Name entry",
                    font.filename.to_string_lossy()
                ))
            })?
            .chars()
            .collect::<String>();
        let style = if font.is_variable_font() {
            "Variable".to_string()
        } else {
            font.font()
                .localized_strings(NameId::TYPOGRAPHIC_SUBFAMILY_NAME)
                .english_or_first()
                .or_else(|| {
                    font.font()
                        .localized_strings(NameId::SUBFAMILY_NAME)
                        .english_or_first()
                })
                .map(|s| s.chars().collect::<String>())
                .unwrap_or_else(|| base_name(&font.filename.to_string_lossy()).to_string())
        };
        families
            .entry(superfamily(&name))
            .or_default()
            .entry(name)
            .or_default()
            .push((
                font.font()
                    .os2()
                    .map(|o| o.us_weight_class())
                    .unwrap_or(400),
                style,
            ));
    }
    let table = |show_root: bool| {
        let mut out = if show_root {
            String::from("| Superfamily | Typographic family | Styles |\n|---|---|---|\n")
        } else {
            String::from("| Typographic family | Styles |\n|---|---|\n")
        };
        for (root, names) in &families {
            for (name, files) in names {
                let mut styles = files.clone();
                styles.sort();
                let names: Vec<&str> = styles.iter().map(|(_, s)| s.as_str()).collect();
                let fonts = format!("{}: {}", names.len(), names.join(", "));
                if show_root {
                    out.push_str(&format!("| **{root}** | {name} | {fonts} |\n"));
                } else {
                    out.push_str(&format!("| {name} | {fonts} |\n"));
                }
            }
        }
        out
    };
    if families.len() > 1 {
        let roots: Vec<String> = families.keys().map(|r| format!("'{r}'")).collect();
        return Ok(Status::just_one_fail(
            "inconsistent-family-name",
            &format!(
                "The fonts belong to {} different families ({}), even after ignoring width and optical-size words:\n\n{}",
                families.len(),
                roots.join(", "),
                table(true)
            ),
        ));
    }
    match families.iter().next() {
        Some((root, names)) if names.len() > 1 => Ok(Status::just_one_info(
            "subfamilies",
            &format!(
                "All fonts belong to the '{root}' superfamily, split into {} width/optical-size subfamilies:\n\n{}",
                names.len(),
                table(false)
            ),
        )),
        _ => Ok(Status::just_one_pass()),
    }
}

fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::superfamily;

    #[test]
    fn strips_width_and_optical_words() {
        assert_eq!(superfamily("Brand Sans Condensed"), "Brand Sans");
        assert_eq!(superfamily("Brand Sans Extra Condensed"), "Brand Sans");
        assert_eq!(superfamily("Brand Text"), "Brand");
        assert_eq!(superfamily("Brand Sans"), "Brand Sans");
        // A family literally named after a subfamily word keeps its name
        assert_eq!(superfamily("Display"), "Display");
    }
}
