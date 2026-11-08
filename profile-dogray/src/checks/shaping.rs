//! Small HarfRust helpers shared by the shaping-based Dogray checks.

use std::str::FromStr;

use fontspector_checkapi::FontspectorError;
use harfrust::{Direction, Feature, Language, Script, ShapeOptions, Shaper, UnicodeBuffer};

/// Builds a HarfRust font from raw font bytes.
pub(crate) fn face(contents: &[u8]) -> Result<harfrust::FontRef<'_>, FontspectorError> {
    harfrust::FontRef::new(contents)
        .map_err(|e| FontspectorError::Shaping(format!("Failed to load font file: {e}")))
}

/// Shapes `text` and returns the glyph IDs in logical (input) order.
///
/// `ot_lang` is an OpenType language system tag such as "ROM" or "URD"
/// (case-insensitive, without padding), or `None` for the default language.
pub(crate) fn shape(
    shaper: &Shaper,
    text: &str,
    script: Script,
    ot_lang: Option<&str>,
    features: &[Feature],
) -> Vec<u32> {
    let rtl = script == harfrust::script::ARABIC;
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    });
    buffer.set_script(script);
    if let Some(lang) = ot_lang {
        // "x-hbot<tag>" selects an OpenType language tag directly.
        if let Ok(language) = Language::from_str(&format!("x-hbot{}", lang.to_lowercase())) {
            buffer.set_language(language);
        }
    }
    let output = shaper.shape(buffer, ShapeOptions::new().features(features));
    let mut infos: Vec<(u32, u32)> = output
        .glyph_infos()
        .iter()
        .map(|g| (g.cluster, g.glyph_id))
        .collect();
    if rtl {
        // HarfRust returns RTL runs in visual order; put them back in logical order.
        infos.reverse();
    }
    infos.into_iter().map(|(_, gid)| gid).collect()
}

/// A feature turned on for the whole run, e.g. `feature("smcp")`.
pub(crate) fn feature(tag: &str) -> Option<Feature> {
    Feature::from_str(tag).ok()
}
