use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use skrifa::raw::TableProvider;

#[check(
    id = "dogray/os2_cap_and_x_height",
    rationale = "
        OS/2 sCapHeight and sxHeight are used by applications (for example for
        superscript and small-cap synthesis, and for CSS font-size-adjust), so
        they must be set to non-zero values, and the x-height cannot exceed the
        cap height.

        Passing does not mean the values are accurate, only that they are set
        plausibly. Combines The Type Founders' os2/cap_height_x_height_non_zero
        and os2/cap_height_exceeds_x_height.
    ",
    proposal = "Dogray profile",
    title = "OS/2 sCapHeight and sxHeight are set and x-height does not exceed cap height"
)]
fn os2_cap_and_x_height(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let os2 = f.font().os2()?;
    let (Some(cap), Some(x)) = (os2.s_cap_height(), os2.sx_height()) else {
        return Ok(Status::just_one_fail(
            "missing-cap-or-x-height",
            "The OS/2 table is older than version 2 and has no sCapHeight/sxHeight fields.",
        ));
    };
    let mut problems = vec![];
    if cap == 0 || x == 0 {
        problems.push(Status::fail(
            "zero-cap-or-x-height",
            &format!("OS/2 sCapHeight is {cap} and sxHeight is {x}; neither may be zero."),
        ));
    } else if x > cap {
        problems.push(Status::fail(
            "x-height-exceeds-cap-height",
            &format!("OS/2 sxHeight ({x}) is greater than sCapHeight ({cap})."),
        ));
    }
    return_result(problems)
}
