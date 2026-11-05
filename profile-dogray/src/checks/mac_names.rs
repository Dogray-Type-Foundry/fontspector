use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use skrifa::raw::TableProvider;

#[check(
    id = "dogray/mac_names",
    rationale = "
        Reports whether the name table contains Mac (platform 1) records.
        This is informational only: it always reports, and never warns or fails.
    ",
    proposal = "Dogray profile",
    title = "Report whether Mac name table entries (platform 1) are present"
)]
fn mac_names(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let mut ids: Vec<u16> = f
        .font()
        .name()?
        .name_record()
        .iter()
        .filter(|rec| rec.platform_id() == 1)
        .map(|rec| rec.name_id().to_u16())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        Ok(Status::just_one_info(
            "no-mac-names",
            "No Mac (platform 1) name records are present.",
        ))
    } else {
        let list = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(Status::just_one_info(
            "mac-names-present",
            &format!("Mac (platform 1) name records are present for name IDs: {list}."),
        ))
    }
}
