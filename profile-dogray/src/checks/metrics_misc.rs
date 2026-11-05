use fontspector_checkapi::{
    pens::BezGlyph, prelude::*, testfont, FileTypeConvert, DEFAULT_LOCATION,
};
use kurbo::Shape;
use skrifa::{raw::TableProvider, MetadataProvider};

#[check(
    id = "dogray/unreasonable_vmtx",
    rationale = "
        A vmtx table describes vertical metrics for vertical typesetting (as in
        CJK). If its heights are 0 or smaller than the glyphs, vertically set
        text overlaps instead of flowing. This tests a few basic Latin glyphs
        (A B C a b c 1 2 3); if their vmtx heights are below their tops, the
        table is most likely wrong everywhere.

        In Glyphs, a horizontal-only font can avoid a vmtx table by setting
        every layer to 'vertWidth: None' and 'vertOrigin: None'. From The Type
        Founders' unreasonable_vmtx.
    ",
    proposal = "Dogray profile",
    title = "vmtx heights are reasonable"
)]
fn unreasonable_vmtx(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let Ok(vmtx) = f.font().vmtx() else {
        return Ok(Status::just_one_skip(
            "no-vmtx",
            "No vmtx table. That's fine unless the font is meant for vertical typesetting.",
        ));
    };
    let charmap = f.font().charmap();
    let mut bad = vec![];
    for c in "ABCabc123".chars() {
        let Some(gid) = charmap.map(c) else { continue };
        let mut pen = BezGlyph::default();
        if f.draw_glyph(gid, &mut pen, DEFAULT_LOCATION).is_err() {
            continue;
        }
        let Some(y_max) = pen
            .iter()
            .filter(|p| !p.elements().is_empty())
            .map(|p| p.bounding_box().y1)
            .reduce(f64::max)
        else {
            continue;
        };
        let height = vmtx.advance(gid).unwrap_or(0);
        if (height as f64) < y_max {
            bad.push(format!(
                "/{} (height {height}, top {})",
                f.glyph_name_for_id_synthesise(gid),
                y_max.ceil()
            ));
        }
    }
    if bad.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Status::just_one_fail(
            "suspicious-vmtx",
            &format!(
                "These glyphs are taller than their vmtx height: {}. Only a few glyphs were tested, so the problem is likely more widespread.",
                bad.join(", ")
            ),
        ))
    }
}

#[check(
    id = "dogray/valid_underline",
    rationale = "
        If the underline thickness (post.underlineThickness) is 0, no underline is
        drawn in some apps, such as Figma. Port of typenetwork/family/valid_underline.
    ",
    proposal = "Dogray profile",
    title = "Underline thickness is set"
)]
fn valid_underline(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let thickness = f.font().post()?.underline_thickness().to_i16();
    Ok(if thickness == 0 {
        Status::just_one_fail(
            "invalid-underline-thickness",
            "post.underlineThickness is 0, so underlines may not render (e.g. in Figma).",
        )
    } else {
        Status::just_one_pass()
    })
}

#[check(
    id = "dogray/valid_strikeout",
    rationale = "
        If the strikeout size (OS/2.yStrikeoutSize) is 0, no strikethrough is
        drawn in some apps, such as Figma. Port of typenetwork/family/valid_strikeout.
    ",
    proposal = "Dogray profile",
    title = "Strikeout size is set"
)]
fn valid_strikeout(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let size = f.font().os2()?.y_strikeout_size();
    Ok(if size == 0 {
        Status::just_one_fail(
            "invalid-strikeout-size",
            "OS/2.yStrikeoutSize is 0, so strikethrough may not render (e.g. in Figma).",
        )
    } else {
        Status::just_one_pass()
    })
}
