mod arabic;
mod character_sets;
#[rustfmt::skip]
mod charset_data;
#[rustfmt::skip]
mod codepage_data;
mod consistent_family_name;
mod currency_symbols;
mod family_names_misc;
mod feature_parity;
mod font_sets;
mod latin_locl;
mod layout;
mod mac_names;
mod metrics_misc;
mod names;
mod os2_heights;
mod os2_ranges;
mod shaping;
mod vertical_metrics;

pub use arabic::{arabic_joining, arabic_support};
pub use character_sets::character_sets;
pub use consistent_family_name::consistent_family_name;
pub use currency_symbols::currency_symbols;
pub use family_names_misc::{equal_codepoint_coverage, fstype, valid_nameid25};
pub use feature_parity::feature_parity;
pub use latin_locl::latin_locl;
pub use layout::{expected_features, gdef_spacing_glyphs_not_marks, languagesystems_check};
pub use mac_names::mac_names;
pub use metrics_misc::{unreasonable_vmtx, valid_strikeout, valid_underline};
pub use names::{
    duplicated_names, family_and_style_max_length, postscript_name_hyphen, style_linking,
};
pub use os2_heights::os2_cap_and_x_height;
pub use os2_ranges::os2_ranges_match_cmap;
pub use vertical_metrics::vertical_metrics;
