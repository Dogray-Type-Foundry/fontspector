//! OS/2 Unicode and code-page ranges, recomputed from the cmap the same way
//! fontmake/fontc (and fontTools' recalcUnicodeRanges/recalcCodePageRanges) do.
//!
//! The range table and the code-page rules below are copied from fontc's
//! `fontbe/src/os2.rs` (Apache-2.0), which ports fontTools.
use std::{cmp::Ordering, collections::HashSet};

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use skrifa::raw::TableProvider;

use super::codepage_data::CODEPAGES;

/// A code page counts as supported when at least this share of its characters is present.
const CODEPAGE_COVERAGE: f64 = 0.92;

/// Three letters per code page, one from each of the main languages it was
/// made for, so supporting them implies the code page's languages. Code pages
/// for the same languages share letters (Mac Roman and the DOS Western pages
/// use the Windows-1252 letters).
fn key_glyphs(bit: u32) -> &'static [char] {
    match bit {
        0 | 29 | 62 => &['ß', 'ñ', 'ø'], // Western European: German, Spanish, Nordic
        63 => &['é', 'ñ', 'ü'],          // DOS US: French, Spanish, German
        1 | 58 => &['ł', 'ř', 'ő'],      // Central European: Polish, Czech, Hungarian
        2 | 57 => &['ы', 'є', 'ђ'],      // Cyrillic: Russian, Ukrainian, Serbian
        49 => &['ы', 'є', 'ў'],          // DOS Russian: Russian, Ukrainian, Belarusian
        3 | 48 => &['ά', 'ω', 'ς'],      // Greek (monotonic)
        60 => &['α', 'ω', 'ς'],          // DOS Greek
        4 | 56 => &['ğ', 'ş', 'ı'],      // Turkish
        5 | 53 => &['א', 'ש', 'ץ'],      // Hebrew
        6 => &['ب', 'پ', 'ے'],           // Arabic: Arabic, Persian, Urdu
        51 => &['ﺏ', 'ﻉ', 'ﻱ'],          // DOS Arabic (presentation forms)
        7 | 59 => &['ė', 'ņ', 'õ'],      // Baltic: Lithuanian, Latvian, Estonian
        8 => &['ơ', 'ư', 'đ'],           // Vietnamese
        16 => &['ก', 'ฮ', 'ะ'],          // Thai
        17 => &['あ', 'ア', '日'],       // Japanese: hiragana, katakana, kanji
        18 => &['们', '这', '国'],       // Simplified Chinese
        19 => &['가', '한', '글'],       // Korean (Wansung)
        20 => &['們', '這', '國'],       // Traditional Chinese
        21 => &['가', '한', '곴'],       // Korean (Johab, all syllables)
        50 => &['ø', 'å', 'æ'],          // DOS Nordic
        52 => &['é', 'ê', 'ç'],          // DOS Canadian French
        54 => &['þ', 'ð', 'ý'],          // DOS Icelandic
        55 => &['ã', 'õ', 'ç'],          // DOS Portuguese
        _ => &[],
    }
}

fn missing_keys(bit: u32, codepoints: &HashSet<u32>) -> Vec<char> {
    key_glyphs(bit)
        .iter()
        .copied()
        .filter(|c| !codepoints.contains(&(*c as u32)))
        .collect()
}

/// Code-page bits the font supports: at least 92% of the code page's
/// characters, including the letters that identify its script. The OEM bit
/// (30), which has no character table, uses fontTools' rule. Falls back to
/// Latin 1 when nothing qualifies, as fontmake does, so the font still works
/// in Word.
fn supported_code_pages(codepoints: &HashSet<u32>) -> HashSet<u32> {
    let mut bits: HashSet<u32> = CODEPAGES
        .iter()
        .filter(|(bit, _, ranges)| {
            coverage(ranges, codepoints) >= CODEPAGE_COVERAGE
                && missing_keys(*bit, codepoints).is_empty()
        })
        .map(|(bit, _, _)| *bit)
        .collect();
    // OEM (30) and ASMO 708 (61) have no reliable character table
    let fonttools_bits = codepage_range_bits(codepoints);
    for bit in [30, 61] {
        if fonttools_bits.contains(&bit) {
            bits.insert(bit);
        }
    }
    if bits.is_empty() {
        bits.insert(0);
    }
    bits
}

/// Why a code-page bit is or isn't supported, for messages.
fn explain(bit: u32, codepoints: &HashSet<u32>) -> String {
    let mut parts = vec![];
    if let Some((_, _, ranges)) = CODEPAGES.iter().find(|(b, _, _)| *b == bit) {
        let pct = coverage(ranges, codepoints) * 100.0;
        parts.push(if pct >= CODEPAGE_COVERAGE * 100.0 {
            format!("{pct:.0}% covered, above the 92% threshold")
        } else {
            format!("only {pct:.0}% covered, below the 92% threshold")
        });
    }
    let missing = missing_keys(bit, codepoints);
    if !missing.is_empty() {
        let list = missing
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        parts.push(format!(
            "missing the identifying letter{} {list}",
            if missing.len() == 1 { "" } else { "s" }
        ));
    }
    let qualifies = CODEPAGES
        .iter()
        .find(|(b, _, _)| *b == bit)
        .is_some_and(|(_, _, ranges)| coverage(ranges, codepoints) >= CODEPAGE_COVERAGE)
        && missing.is_empty();
    if bit == 0 && !qualifies && supported_code_pages(codepoints).contains(&0) {
        parts.push("expected anyway: when no code page qualifies, Latin 1 is set so Word can still use the font".to_string());
    }
    parts.join("; ")
}

/// Share of a code page's characters present in the font.
fn coverage(ranges: &[(u32, u32)], codepoints: &HashSet<u32>) -> f64 {
    let total: u32 = ranges.iter().map(|(a, b)| b - a + 1).sum();
    let have = codepoints
        .iter()
        .filter(|cp| ranges.iter().any(|(a, b)| (*a..=*b).contains(*cp)))
        .count();
    if total == 0 {
        0.0
    } else {
        have as f64 / total as f64
    }
}

fn codepage_name(bit: u32) -> String {
    CODEPAGES
        .iter()
        .find(|(b, _, _)| *b == bit)
        .map_or_else(|| "(reserved)".to_string(), |(_, name, _)| name.to_string())
}

fn unicode_range_name(bit: u32) -> &'static str {
    UNICODE_RANGE_NAMES
        .get(bit as usize)
        .copied()
        .unwrap_or("(reserved)")
}

/// How many of the font's characters fall in Unicode range bit `bit`.
fn chars_in_range(bit: u32, codepoints: &HashSet<u32>) -> usize {
    if bit == 57 {
        // Non-Plane 0: any supplementary-plane character.
        return codepoints.iter().filter(|cp| **cp > 0xFFFF).count();
    }
    codepoints
        .iter()
        .filter(|cp| {
            UNICODE_RANGES
                .iter()
                .any(|(a, b, rbit)| *rbit == bit && (*a..=*b).contains(*cp))
        })
        .count()
}

fn yes_no(set: bool) -> &'static str {
    if set {
        "set"
    } else {
        "not set"
    }
}

/// Names of the OS/2 Unicode range bits, by bit.
///
/// Generated from fontTools' OS2_UNICODE_RANGES.
#[rustfmt::skip]
const UNICODE_RANGE_NAMES: [&str; 123] = [
    "Basic Latin", // 0
    "Latin-1 Supplement", // 1
    "Latin Extended-A", // 2
    "Latin Extended-B", // 3
    "IPA Extensions, Phonetic Extensions, Phonetic Extensions Supplement", // 4
    "Spacing Modifier Letters, Modifier Tone Letters", // 5
    "Combining Diacritical Marks, Combining Diacritical Marks Supplement", // 6
    "Greek and Coptic", // 7
    "Coptic", // 8
    "Cyrillic, Cyrillic Supplement, Cyrillic Extended-A, Cyrillic Extended-B", // 9
    "Armenian", // 10
    "Hebrew", // 11
    "Vai", // 12
    "Arabic, Arabic Supplement", // 13
    "NKo", // 14
    "Devanagari", // 15
    "Bengali", // 16
    "Gurmukhi", // 17
    "Gujarati", // 18
    "Oriya", // 19
    "Tamil", // 20
    "Telugu", // 21
    "Kannada", // 22
    "Malayalam", // 23
    "Thai", // 24
    "Lao", // 25
    "Georgian, Georgian Supplement", // 26
    "Balinese", // 27
    "Hangul Jamo", // 28
    "Latin Extended Additional, Latin Extended-C, Latin Extended-D", // 29
    "Greek Extended", // 30
    "General Punctuation, Supplemental Punctuation", // 31
    "Superscripts And Subscripts", // 32
    "Currency Symbols", // 33
    "Combining Diacritical Marks For Symbols", // 34
    "Letterlike Symbols", // 35
    "Number Forms", // 36
    "Arrows, Supplemental Arrows-A, Supplemental Arrows-B, Miscellaneous Symbols and Arrows", // 37
    "Mathematical Operators, Supplemental Mathematical Operators, Miscellaneous Mathematical Symbols-A, Miscellaneous Mathematical Symbols-B", // 38
    "Miscellaneous Technical", // 39
    "Control Pictures", // 40
    "Optical Character Recognition", // 41
    "Enclosed Alphanumerics", // 42
    "Box Drawing", // 43
    "Block Elements", // 44
    "Geometric Shapes", // 45
    "Miscellaneous Symbols", // 46
    "Dingbats", // 47
    "CJK Symbols And Punctuation", // 48
    "Hiragana", // 49
    "Katakana, Katakana Phonetic Extensions", // 50
    "Bopomofo, Bopomofo Extended", // 51
    "Hangul Compatibility Jamo", // 52
    "Phags-pa", // 53
    "Enclosed CJK Letters And Months", // 54
    "CJK Compatibility", // 55
    "Hangul Syllables", // 56
    "Non-Plane 0", // 57
    "Phoenician", // 58
    "CJK Unified Ideographs, CJK Radicals Supplement, Kangxi Radicals, Ideographic Description Characters, CJK Unified Ideographs Extension A, CJK Unified Ideographs Extension B, Kanbun", // 59
    "Private Use Area (plane 0)", // 60
    "CJK Strokes, CJK Compatibility Ideographs, CJK Compatibility Ideographs Supplement", // 61
    "Alphabetic Presentation Forms", // 62
    "Arabic Presentation Forms-A", // 63
    "Combining Half Marks", // 64
    "Vertical Forms, CJK Compatibility Forms", // 65
    "Small Form Variants", // 66
    "Arabic Presentation Forms-B", // 67
    "Halfwidth And Fullwidth Forms", // 68
    "Specials", // 69
    "Tibetan", // 70
    "Syriac", // 71
    "Thaana", // 72
    "Sinhala", // 73
    "Myanmar", // 74
    "Ethiopic, Ethiopic Supplement, Ethiopic Extended", // 75
    "Cherokee", // 76
    "Unified Canadian Aboriginal Syllabics", // 77
    "Ogham", // 78
    "Runic", // 79
    "Khmer, Khmer Symbols", // 80
    "Mongolian", // 81
    "Braille Patterns", // 82
    "Yi Syllables, Yi Radicals", // 83
    "Tagalog, Hanunoo, Buhid, Tagbanwa", // 84
    "Old Italic", // 85
    "Gothic", // 86
    "Deseret", // 87
    "Byzantine Musical Symbols, Musical Symbols, Ancient Greek Musical Notation", // 88
    "Mathematical Alphanumeric Symbols", // 89
    "Private Use (plane 15), Private Use (plane 16)", // 90
    "Variation Selectors, Variation Selectors Supplement", // 91
    "Tags", // 92
    "Limbu", // 93
    "Tai Le", // 94
    "New Tai Lue", // 95
    "Buginese", // 96
    "Glagolitic", // 97
    "Tifinagh", // 98
    "Yijing Hexagram Symbols", // 99
    "Syloti Nagri", // 100
    "Linear B Syllabary, Linear B Ideograms, Aegean Numbers", // 101
    "Ancient Greek Numbers", // 102
    "Ugaritic", // 103
    "Old Persian", // 104
    "Shavian", // 105
    "Osmanya", // 106
    "Cypriot Syllabary", // 107
    "Kharoshthi", // 108
    "Tai Xuan Jing Symbols", // 109
    "Cuneiform, Cuneiform Numbers and Punctuation", // 110
    "Counting Rod Numerals", // 111
    "Sundanese", // 112
    "Lepcha", // 113
    "Ol Chiki", // 114
    "Saurashtra", // 115
    "Kayah Li", // 116
    "Rejang", // 117
    "Cham", // 118
    "Ancient Symbols", // 119
    "Phaistos Disc", // 120
    "Carian, Lycian, Lydian", // 121
    "Domino Tiles, Mahjong Tiles", // 122
];

/// formed by .0..=.1; if it matches bit .2 should be set.
///
/// Generated by resources/scripts/os2_unicode_ranges.py.
///
/// <https://learn.microsoft.com/en-us/typography/opentype/spec/os2#ur>
/// <https://github.com/fonttools/fonttools/blob/47813b217c1f8bc343c094776020b4f32fc024b0/Lib/fontTools/ttLib/tables/O_S_2f_2.py#L351-L557>
const UNICODE_RANGES: &[(u32, u32, u32)] = &[
    (0x0000, 0x007F, 0),      // Basic Latin
    (0x0080, 0x00FF, 1),      // Latin-1 Supplement
    (0x0100, 0x017F, 2),      // Latin Extended-A
    (0x0180, 0x024F, 3),      // Latin Extended-B
    (0x0250, 0x02AF, 4),      // IPA Extensions
    (0x02B0, 0x02FF, 5),      // Spacing Modifier Letters
    (0x0300, 0x036F, 6),      // Combining Diacritical Marks
    (0x0370, 0x03FF, 7),      // Greek and Coptic
    (0x0400, 0x04FF, 9),      // Cyrillic
    (0x0500, 0x052F, 9),      // Cyrillic Supplement
    (0x0530, 0x058F, 10),     // Armenian
    (0x0590, 0x05FF, 11),     // Hebrew
    (0x0600, 0x06FF, 13),     // Arabic
    (0x0700, 0x074F, 71),     // Syriac
    (0x0750, 0x077F, 13),     // Arabic Supplement
    (0x0780, 0x07BF, 72),     // Thaana
    (0x07C0, 0x07FF, 14),     // NKo
    (0x0900, 0x097F, 15),     // Devanagari
    (0x0980, 0x09FF, 16),     // Bengali
    (0x0A00, 0x0A7F, 17),     // Gurmukhi
    (0x0A80, 0x0AFF, 18),     // Gujarati
    (0x0B00, 0x0B7F, 19),     // Oriya
    (0x0B80, 0x0BFF, 20),     // Tamil
    (0x0C00, 0x0C7F, 21),     // Telugu
    (0x0C80, 0x0CFF, 22),     // Kannada
    (0x0D00, 0x0D7F, 23),     // Malayalam
    (0x0D80, 0x0DFF, 73),     // Sinhala
    (0x0E00, 0x0E7F, 24),     // Thai
    (0x0E80, 0x0EFF, 25),     // Lao
    (0x0F00, 0x0FFF, 70),     // Tibetan
    (0x1000, 0x109F, 74),     // Myanmar
    (0x10A0, 0x10FF, 26),     // Georgian
    (0x1100, 0x11FF, 28),     // Hangul Jamo
    (0x1200, 0x137F, 75),     // Ethiopic
    (0x1380, 0x139F, 75),     // Ethiopic Supplement
    (0x13A0, 0x13FF, 76),     // Cherokee
    (0x1400, 0x167F, 77),     // Unified Canadian Aboriginal Syllabics
    (0x1680, 0x169F, 78),     // Ogham
    (0x16A0, 0x16FF, 79),     // Runic
    (0x1700, 0x171F, 84),     // Tagalog
    (0x1720, 0x173F, 84),     // Hanunoo
    (0x1740, 0x175F, 84),     // Buhid
    (0x1760, 0x177F, 84),     // Tagbanwa
    (0x1780, 0x17FF, 80),     // Khmer
    (0x1800, 0x18AF, 81),     // Mongolian
    (0x1900, 0x194F, 93),     // Limbu
    (0x1950, 0x197F, 94),     // Tai Le
    (0x1980, 0x19DF, 95),     // New Tai Lue
    (0x19E0, 0x19FF, 80),     // Khmer Symbols
    (0x1A00, 0x1A1F, 96),     // Buginese
    (0x1B00, 0x1B7F, 27),     // Balinese
    (0x1B80, 0x1BBF, 112),    // Sundanese
    (0x1C00, 0x1C4F, 113),    // Lepcha
    (0x1C50, 0x1C7F, 114),    // Ol Chiki
    (0x1D00, 0x1D7F, 4),      // Phonetic Extensions
    (0x1D80, 0x1DBF, 4),      // Phonetic Extensions Supplement
    (0x1DC0, 0x1DFF, 6),      // Combining Diacritical Marks Supplement
    (0x1E00, 0x1EFF, 29),     // Latin Extended Additional
    (0x1F00, 0x1FFF, 30),     // Greek Extended
    (0x2000, 0x206F, 31),     // General Punctuation
    (0x2070, 0x209F, 32),     // Superscripts And Subscripts
    (0x20A0, 0x20CF, 33),     // Currency Symbols
    (0x20D0, 0x20FF, 34),     // Combining Diacritical Marks For Symbols
    (0x2100, 0x214F, 35),     // Letterlike Symbols
    (0x2150, 0x218F, 36),     // Number Forms
    (0x2190, 0x21FF, 37),     // Arrows
    (0x2200, 0x22FF, 38),     // Mathematical Operators
    (0x2300, 0x23FF, 39),     // Miscellaneous Technical
    (0x2400, 0x243F, 40),     // Control Pictures
    (0x2440, 0x245F, 41),     // Optical Character Recognition
    (0x2460, 0x24FF, 42),     // Enclosed Alphanumerics
    (0x2500, 0x257F, 43),     // Box Drawing
    (0x2580, 0x259F, 44),     // Block Elements
    (0x25A0, 0x25FF, 45),     // Geometric Shapes
    (0x2600, 0x26FF, 46),     // Miscellaneous Symbols
    (0x2700, 0x27BF, 47),     // Dingbats
    (0x27C0, 0x27EF, 38),     // Miscellaneous Mathematical Symbols-A
    (0x27F0, 0x27FF, 37),     // Supplemental Arrows-A
    (0x2800, 0x28FF, 82),     // Braille Patterns
    (0x2900, 0x297F, 37),     // Supplemental Arrows-B
    (0x2980, 0x29FF, 38),     // Miscellaneous Mathematical Symbols-B
    (0x2A00, 0x2AFF, 38),     // Supplemental Mathematical Operators
    (0x2B00, 0x2BFF, 37),     // Miscellaneous Symbols and Arrows
    (0x2C00, 0x2C5F, 97),     // Glagolitic
    (0x2C60, 0x2C7F, 29),     // Latin Extended-C
    (0x2C80, 0x2CFF, 8),      // Coptic
    (0x2D00, 0x2D2F, 26),     // Georgian Supplement
    (0x2D30, 0x2D7F, 98),     // Tifinagh
    (0x2D80, 0x2DDF, 75),     // Ethiopic Extended
    (0x2DE0, 0x2DFF, 9),      // Cyrillic Extended-A
    (0x2E00, 0x2E7F, 31),     // Supplemental Punctuation
    (0x2E80, 0x2EFF, 59),     // CJK Radicals Supplement
    (0x2F00, 0x2FDF, 59),     // Kangxi Radicals
    (0x2FF0, 0x2FFF, 59),     // Ideographic Description Characters
    (0x3000, 0x303F, 48),     // CJK Symbols And Punctuation
    (0x3040, 0x309F, 49),     // Hiragana
    (0x30A0, 0x30FF, 50),     // Katakana
    (0x3100, 0x312F, 51),     // Bopomofo
    (0x3130, 0x318F, 52),     // Hangul Compatibility Jamo
    (0x3190, 0x319F, 59),     // Kanbun
    (0x31A0, 0x31BF, 51),     // Bopomofo Extended
    (0x31C0, 0x31EF, 61),     // CJK Strokes
    (0x31F0, 0x31FF, 50),     // Katakana Phonetic Extensions
    (0x3200, 0x32FF, 54),     // Enclosed CJK Letters And Months
    (0x3300, 0x33FF, 55),     // CJK Compatibility
    (0x3400, 0x4DBF, 59),     // CJK Unified Ideographs Extension A
    (0x4DC0, 0x4DFF, 99),     // Yijing Hexagram Symbols
    (0x4E00, 0x9FFF, 59),     // CJK Unified Ideographs
    (0xA000, 0xA48F, 83),     // Yi Syllables
    (0xA490, 0xA4CF, 83),     // Yi Radicals
    (0xA500, 0xA63F, 12),     // Vai
    (0xA640, 0xA69F, 9),      // Cyrillic Extended-B
    (0xA700, 0xA71F, 5),      // Modifier Tone Letters
    (0xA720, 0xA7FF, 29),     // Latin Extended-D
    (0xA800, 0xA82F, 100),    // Syloti Nagri
    (0xA840, 0xA87F, 53),     // Phags-pa
    (0xA880, 0xA8DF, 115),    // Saurashtra
    (0xA900, 0xA92F, 116),    // Kayah Li
    (0xA930, 0xA95F, 117),    // Rejang
    (0xAA00, 0xAA5F, 118),    // Cham
    (0xAC00, 0xD7AF, 56),     // Hangul Syllables
    (0xD800, 0xDFFF, 57),     // Non-Plane 0 *
    (0xE000, 0xF8FF, 60),     // Private Use Area (plane 0)
    (0xF900, 0xFAFF, 61),     // CJK Compatibility Ideographs
    (0xFB00, 0xFB4F, 62),     // Alphabetic Presentation Forms
    (0xFB50, 0xFDFF, 63),     // Arabic Presentation Forms-A
    (0xFE00, 0xFE0F, 91),     // Variation Selectors
    (0xFE10, 0xFE1F, 65),     // Vertical Forms
    (0xFE20, 0xFE2F, 64),     // Combining Half Marks
    (0xFE30, 0xFE4F, 65),     // CJK Compatibility Forms
    (0xFE50, 0xFE6F, 66),     // Small Form Variants
    (0xFE70, 0xFEFF, 67),     // Arabic Presentation Forms-B
    (0xFF00, 0xFFEF, 68),     // Halfwidth And Fullwidth Forms
    (0xFFF0, 0xFFFF, 69),     // Specials
    (0x10000, 0x1007F, 101),  // Linear B Syllabary
    (0x10080, 0x100FF, 101),  // Linear B Ideograms
    (0x10100, 0x1013F, 101),  // Aegean Numbers
    (0x10140, 0x1018F, 102),  // Ancient Greek Numbers
    (0x10190, 0x101CF, 119),  // Ancient Symbols
    (0x101D0, 0x101FF, 120),  // Phaistos Disc
    (0x10280, 0x1029F, 121),  // Lycian
    (0x102A0, 0x102DF, 121),  // Carian
    (0x10300, 0x1032F, 85),   // Old Italic
    (0x10330, 0x1034F, 86),   // Gothic
    (0x10380, 0x1039F, 103),  // Ugaritic
    (0x103A0, 0x103DF, 104),  // Old Persian
    (0x10400, 0x1044F, 87),   // Deseret
    (0x10450, 0x1047F, 105),  // Shavian
    (0x10480, 0x104AF, 106),  // Osmanya
    (0x10800, 0x1083F, 107),  // Cypriot Syllabary
    (0x10900, 0x1091F, 58),   // Phoenician
    (0x10920, 0x1093F, 121),  // Lydian
    (0x10A00, 0x10A5F, 108),  // Kharoshthi
    (0x12000, 0x123FF, 110),  // Cuneiform
    (0x12400, 0x1247F, 110),  // Cuneiform Numbers and Punctuation
    (0x1D000, 0x1D0FF, 88),   // Byzantine Musical Symbols
    (0x1D100, 0x1D1FF, 88),   // Musical Symbols
    (0x1D200, 0x1D24F, 88),   // Ancient Greek Musical Notation
    (0x1D300, 0x1D35F, 109),  // Tai Xuan Jing Symbols
    (0x1D360, 0x1D37F, 111),  // Counting Rod Numerals
    (0x1D400, 0x1D7FF, 89),   // Mathematical Alphanumeric Symbols
    (0x1F000, 0x1F02F, 122),  // Mahjong Tiles
    (0x1F030, 0x1F09F, 122),  // Domino Tiles
    (0x20000, 0x2A6DF, 59),   // CJK Unified Ideographs Extension B
    (0x2F800, 0x2FA1F, 61),   // CJK Compatibility Ideographs Supplement
    (0xE0000, 0xE007F, 92),   // Tags
    (0xE0100, 0xE01EF, 91),   // Variation Selectors Supplement
    (0xF0000, 0xFFFFD, 90),   // Private Use (plane 15)
    (0x100000, 0x10FFFD, 90), // Private Use (plane 16)
];

fn add_unicode_range_bits(add_to: &mut HashSet<u32>, codepoint: u32) {
    let maybe_idx = UNICODE_RANGES
        .binary_search_by(|(from, to, _)| match codepoint {
            _ if codepoint < *from => Ordering::Greater,
            _ if codepoint > *to => Ordering::Less,
            _ => Ordering::Equal,
        })
        .ok();
    if let Some((_, _, bit)) = maybe_idx.and_then(|idx| UNICODE_RANGES.get(idx)) {
        add_to.insert(*bit);
    }

    // The spec says that bit 57 ("Non Plane 0") is set for any codepoint beyond BMP so do that too
    // https://github.com/fonttools/fonttools/blob/47813b217c1f8bc343c094776020b4f32fc024b0/Lib/fontTools/ttLib/tables/O_S_2f_2.py#L599-L603
    if (0x10000..=0x10FFFF).contains(&codepoint) {
        add_to.insert(57);
    }
}

fn codepage_range_bits(codepoints: &HashSet<u32>) -> HashSet<u32> {
    let mut bits = HashSet::new();

    let chars = codepoints
        .iter()
        .filter_map(|cp| char::from_u32(*cp))
        .collect::<HashSet<_>>();
    let has_ascii = (0x20_u32..0x7E).all(|cp| codepoints.contains(&cp));
    let has_lineart = chars.contains(&'┤');

    for char in chars.iter() {
        match char {
            'Þ' if has_ascii => {
                bits.insert(0);
            } //Latin 1
            'Ľ' if has_ascii => {
                bits.insert(1); //Latin 2: Eastern Europe
                if has_lineart {
                    bits.insert(58); //Latin 2
                }
            }
            'Б' => {
                bits.insert(2); //Cyrillic
                if chars.contains(&'Ѕ') && has_lineart {
                    bits.insert(57); //IBM Cyrillic
                }
                if chars.contains(&'╜') && has_lineart {
                    bits.insert(49); //MS-DOS Russian
                }
            }
            'Ά' => {
                bits.insert(3); //Greek
                if has_lineart && chars.contains(&'½') {
                    bits.insert(48); //IBM Greek
                }
                if has_lineart && chars.contains(&'√') {
                    bits.insert(60); //Greek, former 437 G
                }
            }
            'İ' if has_ascii => {
                bits.insert(4); //Turkish
                if has_lineart {
                    bits.insert(56); //IBM turkish
                }
            }
            'א' => {
                bits.insert(5); //Hebrew
                if has_lineart && chars.contains(&'√') {
                    bits.insert(53); //Hebrew
                }
            }
            'ر' => {
                bits.insert(6); //Arabic
                if chars.contains(&'√') {
                    bits.insert(51); //Arabic
                }
                if has_lineart {
                    bits.insert(61); //Arabic; ASMO 708
                }
            }
            'ŗ' if has_ascii => {
                bits.insert(7); //Windows Baltic
                if has_lineart {
                    bits.insert(59); //MS-DOS Baltic
                }
            }
            '₫' if has_ascii => {
                bits.insert(8); //Vietnamese
            }
            'ๅ' => {
                bits.insert(16); //Thai
            }
            'エ' => {
                bits.insert(17); //JIS/Japan
            }
            'ㄅ' => {
                bits.insert(18); //Chinese: Simplified chars
            }
            'ㄱ' => {
                bits.insert(19); //Korean wansung
            }
            '央' => {
                bits.insert(20); //Chinese: Traditional chars
            }
            '곴' => {
                bits.insert(21); //Korean Johab
            }
            '♥' if has_ascii => {
                bits.insert(30); //OEM Character Set
            }
            //    //TODO: Symbol bit has a special meaning (check the spec), we need
            //    //to confirm if this is wanted by default.
            //    //elif  chr(0xF000) <= char <= chr(0xF0FF) {
            //    //   bits.insert(31)         //Symbol Character Set
            'þ' if has_ascii && has_lineart => {
                bits.insert(54); //MS-DOS Icelandic
            }
            '╚' if has_ascii => {
                bits.insert(62); //WE/Latin 1
                bits.insert(63); //US
            }
            'Å' if has_ascii && has_lineart && chars.contains(&'√') => {
                bits.insert(50); //MS-DOS Nordic
            }
            'é' if has_ascii && has_lineart && chars.contains(&'√') => {
                bits.insert(52); //MS-DOS Canadian French
            }
            'õ' if has_ascii && has_lineart && chars.contains(&'√') => {
                bits.insert(55); //MS-DOS Portuguese
            }
            _ => (),
        }
    }

    if has_ascii && chars.contains(&'‰') && chars.contains(&'∑') {
        bits.insert(29); // Macintosh Character Set (US Roman)
    }

    // when no codepage ranges can be enabled, fall back to enabling bit 0
    // (Latin 1) so that the font works in MS Word:
    // https://github.com/googlei18n/fontmake/issues/468
    if bits.is_empty() {
        bits.insert(0);
    }

    bits
}

fn bits_of(words: &[u32]) -> HashSet<u32> {
    words
        .iter()
        .enumerate()
        .flat_map(|(word, value)| {
            (0..32u32)
                .filter(move |bit| value & (1 << bit) != 0)
                .map(move |bit| word as u32 * 32 + bit)
        })
        .collect()
}

/// "0 Latin 1, 1 Latin 2", or "none".
fn describe_named(bits: &HashSet<u32>, name: fn(u32) -> String) -> String {
    let mut v: Vec<u32> = bits.iter().copied().collect();
    v.sort_unstable();
    if v.is_empty() {
        "none".to_string()
    } else {
        v.iter()
            .map(|b| format!("{b} {}", name(*b)))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[check(
    id = "dogray/os2_ranges_match_cmap",
    rationale = "
        The OS/2 Unicode range (ulUnicodeRange1-4) and code-page range
        (ulCodePageRange1-2) bits must agree with the font's cmap. Wrong values
        can affect font classification, selection and rendering in some apps; with
        no code-page bits at all, Word and other apps under Windows may refuse to
        use the font for any language.

        Unicode range bits are recomputed from the cmap with the same rules
        fontmake and fontc use (ported from fontTools). A code-page bit should be
        set when the font has at least 92% of that code page's characters,
        including three identifying letters, one from each of the main
        languages the code page was made for: ß ñ ø for Western European
        (Windows-1252 and Mac Roman alike), ł ř ő for Central European, ы є ђ
        for Cyrillic, ė ņ õ for Baltic, ơ ư đ for Vietnamese, and so on.

        Based on The Type Founders' os2/ranges_match_cmap; replaces
        opentype/code_pages, which only failed when no code page was set.
    ",
    proposal = "Dogray profile",
    title = "OS/2 Unicode and code-page ranges match the cmap"
)]
fn os2_ranges_match_cmap(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let os2 = f.font().os2()?;
    let codepoints = f.codepoints(Some(context));

    let mut expected_unicode = HashSet::new();
    for cp in codepoints.iter() {
        add_unicode_range_bits(&mut expected_unicode, *cp);
    }
    let actual_unicode = bits_of(&[
        os2.ul_unicode_range_1(),
        os2.ul_unicode_range_2(),
        os2.ul_unicode_range_3(),
        os2.ul_unicode_range_4(),
    ]);
    let expected_code_pages = supported_code_pages(&codepoints);
    let actual_code_pages = bits_of(&[
        os2.ul_code_page_range_1().unwrap_or(0),
        os2.ul_code_page_range_2().unwrap_or(0),
    ]);

    let mut problems = vec![];
    if actual_code_pages != expected_code_pages {
        let mut bits: Vec<u32> = actual_code_pages
            .symmetric_difference(&expected_code_pages)
            .copied()
            .collect();
        bits.sort_unstable();
        problems.push(Status::fail(
            "code-page-ranges-mismatch",
            &format!(
                "OS/2 code-page bits that don't match the characters the font supports ({}). A code page counts as supported when the font has at least 92% of its characters, including its three identifying letters.\n\n{}\n\nShould be set: {}.",
                bits.len(),
                markdown_table(
                    context,
                    &["Bit", "Code page", "Now", "Should be", "Why"],
                    bits.iter().map(|bit| {
                        vec![
                            bit.to_string(),
                            codepage_name(*bit),
                            yes_no(actual_code_pages.contains(bit)).to_string(),
                            yes_no(expected_code_pages.contains(bit)).to_string(),
                            explain(*bit, &codepoints),
                        ]
                    })
                ),
                describe_named(&expected_code_pages, codepage_name),
            ),
        ));
    }
    if actual_unicode != expected_unicode {
        let mut bits: Vec<u32> = actual_unicode
            .symmetric_difference(&expected_unicode)
            .copied()
            .collect();
        bits.sort_unstable();
        problems.push(Status::fail(
            "unicode-ranges-mismatch",
            &format!(
                "OS/2 Unicode range bits that don't match the cmap ({}). A bit should be set when the font has any character in its range.\n\n{}",
                bits.len(),
                markdown_table(
                    context,
                    &["Bit", "Unicode range", "Now", "Should be", "Characters in font"],
                    bits.iter().map(|bit| {
                        vec![
                            bit.to_string(),
                            unicode_range_name(*bit).to_string(),
                            yes_no(actual_unicode.contains(bit)).to_string(),
                            yes_no(expected_unicode.contains(bit)).to_string(),
                            chars_in_range(*bit, &codepoints).to_string(),
                        ]
                    })
                ),
            ),
        ));
    }
    return_result(problems)
}

#[cfg(test)]
mod tests {
    use super::{key_glyphs, CODEPAGES};

    #[test]
    fn identifying_letters_belong_to_their_code_page() {
        for (bit, name, ranges) in CODEPAGES {
            let keys = key_glyphs(*bit);
            assert_eq!(
                keys.len(),
                3,
                "{name} should have three identifying letters"
            );
            for c in keys {
                let cp = *c as u32;
                assert!(
                    ranges.iter().any(|(a, b)| (*a..=*b).contains(&cp)),
                    "{c} is not in {name}"
                );
            }
        }
    }
}
