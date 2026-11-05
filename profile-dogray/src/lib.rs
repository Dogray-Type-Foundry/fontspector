//! The Dogray profile: a customized copy of the Universal profile, plus
//! Dogray-specific checks. The profile itself lives in `profile.toml` so it can
//! be edited without touching Rust code; this crate registers the extra checks
//! it refers to.
pub mod checks;

use fontspector_checkapi::{FontspectorError, Profile, Registry};

pub struct Dogray;

impl fontspector_checkapi::ProfileProvider for Dogray {
    fn register(&self, cr: &mut Registry) -> Result<(), FontspectorError> {
        // Dogray uses some Adobe and Microsoft checks; those profiles are plugins
        // in the CLI, so register them here if the host hasn't already.
        if cr.get_profile("adobefonts").is_none() {
            profile_adobe::Adobe.register(cr)?;
        }
        if cr.get_profile("microsoft").is_none() {
            profile_microsoft::Microsoft.register(cr)?;
        }

        cr.register_check(checks::vertical_metrics);
        cr.register_check(checks::mac_names);
        cr.register_check(checks::currency_symbols);
        cr.register_check(checks::consistent_family_name);
        cr.register_check(checks::character_sets);
        cr.register_check(checks::os2_cap_and_x_height);
        cr.register_check(checks::os2_ranges_match_cmap);
        cr.register_check(checks::postscript_name_hyphen);
        cr.register_check(checks::family_and_style_max_length);
        cr.register_check(checks::duplicated_names);
        cr.register_check(checks::style_linking);
        cr.register_check(checks::expected_features);
        cr.register_check(checks::languagesystems_check);
        cr.register_check(checks::gdef_spacing_glyphs_not_marks);
        cr.register_check(checks::unreasonable_vmtx);
        cr.register_check(checks::valid_underline);
        cr.register_check(checks::valid_strikeout);
        cr.register_check(checks::fstype);
        cr.register_check(checks::valid_nameid25);
        cr.register_check(checks::equal_codepoint_coverage);

        let profile = Profile::from_toml(include_str!("../profile.toml"))
            .map_err(|e| FontspectorError::General(format!("Invalid Dogray profile: {e}")))?;
        cr.register_profile("dogray", profile, true)
    }
}
