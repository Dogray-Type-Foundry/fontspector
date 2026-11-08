use hashbrown::HashSet;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert, Metadata};
use kurbo::ParamCurve;

use crate::checks::outline::{name_and_bezglyph, seg_ends, seg_kind, seg_span};

#[check(
    id = "overlapping_path_segments",
    rationale = "
        
        Some rasterizers encounter difficulties when rendering glyphs with
        overlapping path segments.

        A path segment is a section of a path defined by two on-curve points.
        When two segments share the same coordinates, they are considered
        overlapping.
    
    ",
    proposal = "https://github.com/google/fonts/issues/7594#issuecomment-2401909084",
    title = "Check there are no overlapping path segments"
)]
fn overlapping_path_segments(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let mut all_warnings = vec![];
    for (glyph_id, (name, result)) in name_and_bezglyph(&f).enumerate() {
        let mut seen = HashSet::new();
        let pen = result?;
        for contour in pen.iter() {
            for seg in contour.segments() {
                let start = seg.eval(0.0).round();
                let end = seg.eval(1.0).round();
                let normal = vec![
                    start.x.to_bits(),
                    start.y.to_bits(),
                    end.x.to_bits(),
                    end.y.to_bits(),
                ];
                let flipped = vec![
                    end.x.to_bits(),
                    end.y.to_bits(),
                    start.x.to_bits(),
                    start.y.to_bits(),
                ];
                if seen.contains(&normal) || seen.contains(&flipped) {
                    all_warnings.push((name.clone(), glyph_id as u32, seg));
                }
                seen.insert(normal);
            }
        }
    }
    if all_warnings.is_empty() {
        return Ok(Status::just_one_pass());
    }
    let mut status = Status::warn(
        "overlapping-path-segments",
        &format!(
            "Segments lying exactly on top of an earlier segment of the same glyph ({}):\n\n{}",
            all_warnings.len(),
            markdown_table(
                context,
                &["Glyph", "Segment", "Kind"],
                all_warnings.iter().map(|(name, _, seg)| vec![
                    name.clone(),
                    seg_span(seg),
                    seg_kind(seg).to_string()
                ])
            )
        ),
    );
    for (glyph_name, glyph_id, seg) in all_warnings {
        let (start, _) = seg_ends(&seg);
        status.add_metadata(Metadata::GlyphProblem {
            glyph_name,
            glyph_id,
            position: Some((start.x as f32, start.y as f32)),
            message: "Overlapping segment".to_string(),
            actual: None,
            expected: None,
            userspace_location: None,
        });
    }
    Ok(Box::new(vec![status].into_iter()))
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::codetesting::{
        assert_pass, assert_results_contain, run_check, test_able,
    };

    use fontspector_checkapi::StatusCode;

    #[test]
    fn test_overlapping_path_segments() {
        let testable = test_able("overlapping_path_segments/Figtree[wght].ttf");
        let results = run_check(super::overlapping_path_segments, testable);
        assert_results_contain(
            &results,
            StatusCode::Warn,
            Some("overlapping-path-segments".to_string()),
        );

        let testable = test_able("merriweather/Merriweather-Regular.ttf");
        let results = run_check(super::overlapping_path_segments, testable);
        assert_pass(&results);
    }
}
