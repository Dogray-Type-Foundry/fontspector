use super::{name_and_bezglyph, pt};
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert, Metadata};
use kurbo::{Rect, Shape};

// Although this check is per-glyph, the problem of contours being oriented
// the wrong way by the compiler tends to affect all glyphs in a font.
// It's generally caused by something like passing --keep-direction in
// fontmake when doing things with cubic sources.
#[check(
    id = "outline_direction",
    rationale = "
        
        In TrueType fonts, the outermost contour of a glyph should be oriented
        clockwise, while the inner contours should be oriented counter-clockwise.
        Getting the path direction wrong can lead to rendering issues in some
        software.
    
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/2056",
    title = "Check the direction of the outermost contour in each glyph"
)]
fn direction(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);

    let is_ttf = f.has_table(b"glyf");

    let direction_is_wrong = |path: &kurbo::BezPath| {
        if is_ttf {
            path.area() > 0.0
        } else {
            path.area() < 0.0
        }
    };
    let wrong_direction = if is_ttf {
        "counter-clockwise"
    } else {
        "clockwise"
    };
    let mut problems = vec![];
    let mut all_warnings = vec![];
    for (glyph_id, (name, result)) in name_and_bezglyph(&f).enumerate() {
        let pen = result?;
        let bounds: Vec<Rect> = pen.iter().map(|path| path.bounding_box()).collect();
        let mut is_within = vec![vec![]; bounds.len()];
        for (i, my_bounds) in bounds.iter().enumerate() {
            if my_bounds.is_zero_area() {
                all_warnings.push((
                    name.clone(),
                    glyph_id as u32,
                    "contour has no area (probably a stray single point)".to_string(),
                ));
                continue;
            }
            for (j, their_bounds) in bounds.iter().enumerate() {
                if i == j {
                    continue;
                }
                if their_bounds.is_zero_area() {
                    continue;
                }
                if my_bounds.contains_rect(*their_bounds) {
                    #[allow(clippy::indexing_slicing)]
                    // is_within is initialized with the same length as bounds
                    is_within[j].push(i);
                }
            }
        }
        for (i, path) in pen.iter().enumerate() {
            #[allow(clippy::indexing_slicing)]
            // is_within is initialized with the same length as bounds
            if is_within[i].is_empty() && direction_is_wrong(path) {
                let start = path
                    .elements()
                    .first()
                    .and_then(|el| el.end_point())
                    .map(pt)
                    .unwrap_or_default();
                all_warnings.push((
                    name.clone(),
                    glyph_id as u32,
                    format!("{wrong_direction} outer contour starting at {start}"),
                ));
            }
        }
    }
    if !all_warnings.is_empty() {
        let mut status = Status::warn(
            "ccw-outer-contour", // Keep status code for compat, even though we detect cw outer contours in CFF fonts
            &format!(
                "Contours running the wrong way; outer contours in {} fonts must not be {wrong_direction} ({}):\n\n{}",
                if is_ttf { "TrueType" } else { "CFF" },
                all_warnings.len(),
                markdown_table(
                    context,
                    &["Glyph", "Problem"],
                    all_warnings.iter().map(|(name, _, problem)| vec![name.clone(), problem.clone()])
                )
            ),
        );
        for (glyph_name, glyph_id, problem) in all_warnings {
            status.add_metadata(Metadata::GlyphProblem {
                glyph_name,
                glyph_id,
                position: None,
                message: problem,
                actual: None,
                expected: None,
                userspace_location: None,
            });
        }
        problems.push(status);
    }

    return_result(problems)
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::codetesting::{
        assert_messages_contain, assert_pass, assert_results_contain, run_check, test_able,
    };

    use fontspector_checkapi::StatusCode;

    #[test]
    fn test_outline_direction() {
        let testable = test_able("wonky_paths/WonkySourceSansPro-Regular.ttf");
        let results = run_check(super::direction, testable);
        assert_results_contain(
            &results,
            StatusCode::Warn,
            Some("ccw-outer-contour".to_string()),
        );
        assert_messages_contain(
            &results,
            "| A (U+0041) | counter-clockwise outer contour starting at",
        );
        assert_messages_contain(
            &results,
            "| x (U+0078) | contour has no area (probably a stray single point) |",
        );

        let testable = test_able("wonky_paths/OutlineTest.ttf");
        let results = run_check(super::direction, testable);
        assert_pass(&results);
    }
}
