use fontspector_checkapi::{pens::BezGlyph, FontspectorError, TestFont, DEFAULT_LOCATION};
use skrifa::MetadataProvider;
use std::ops::Sub;

mod alignment_miss;
mod colinear_vectors;
mod direction;
mod jaggy_segments;
mod overlapping_path_segments;
mod semi_vertical;
mod short_segments;
pub use alignment_miss::alignment_miss;
pub use colinear_vectors::colinear_vectors;
pub use direction::direction;
pub use jaggy_segments::jaggy_segments;
pub use overlapping_path_segments::overlapping_path_segments;
pub use semi_vertical::semi_vertical;
pub use short_segments::short_segments;

pub(crate) fn close_but_not_on<T>(expected: T, actual: T, epsilon: T) -> bool
where
    T: Sub<Output = T> + PartialOrd + Copy + num_traits::sign::Signed,
{
    (actual - expected).abs() <= epsilon && actual != expected
}

pub(crate) fn name_and_bezglyph<'a>(
    f: &'a TestFont,
) -> impl Iterator<Item = (String, Result<BezGlyph, FontspectorError>)> + 'a {
    let reverse_char_map = f
        .font()
        .charmap()
        .mappings()
        .map(|(cp, gid)| (gid, cp))
        .collect::<std::collections::HashMap<_, _>>();
    f.all_glyphs().map(move |glyph| {
        let mut name = f.glyph_name_for_id_synthesise(glyph);
        if let Some(cp) = reverse_char_map.get(&glyph) {
            name = format!("{name} (U+{cp:04X})");
        }
        let mut pen = BezGlyph::default();
        let result = f.draw_glyph(glyph, &mut pen, DEFAULT_LOCATION);
        (name, result.map(|_| pen))
    })
}

/// A coordinate without a pointless ".0" (470.0 -> "470", 39.5 -> "39.5").
pub(crate) fn num(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

/// "(x, y)"
pub(crate) fn pt(p: kurbo::Point) -> String {
    format!("({}, {})", num(p.x), num(p.y))
}

/// Start and end points of a segment.
pub(crate) fn seg_ends(seg: &kurbo::PathSeg) -> (kurbo::Point, kurbo::Point) {
    match seg {
        kurbo::PathSeg::Line(line) => (line.p0, line.p1),
        kurbo::PathSeg::Quad(quad) => (quad.p0, quad.p2),
        kurbo::PathSeg::Cubic(cubic) => (cubic.p0, cubic.p3),
    }
}

/// "line" or "curve"
pub(crate) fn seg_kind(seg: &kurbo::PathSeg) -> &'static str {
    match seg {
        kurbo::PathSeg::Line(_) => "line",
        _ => "curve",
    }
}

/// "(x0, y0) → (x1, y1)"
pub(crate) fn seg_span(seg: &kurbo::PathSeg) -> String {
    let (a, b) = seg_ends(seg);
    format!("{} → {}", pt(a), pt(b))
}
