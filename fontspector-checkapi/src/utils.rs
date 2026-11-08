use std::fmt::Display;

use crate::{CheckFnResult, Context, Status, StatusCode};

/// Formats a list of items as a Markdown bullet list.
pub fn bullet_list<I>(context: &Context, items: I) -> String
where
    I: IntoIterator,
    I::Item: Display,
{
    let mut items = items.into_iter();
    let first_nine = items.by_ref().take(9);
    let mut list = first_nine
        .map(|item| format!("* {item}"))
        .collect::<Vec<_>>();

    if context.full_lists {
        list.extend(items.map(|item| format!("* {item}")));
    } else {
        let remainder = items.count();
        if remainder > 0 {
            list.push(format!("... and {remainder} others"));
        }
    }
    list.join("\n")
}

/// Formats items as one comma-separated line: compact for long lists of
/// glyph names. Unless `context.full_lists` is set, only the first 60 items
/// are shown, followed by a count of the rest.
pub fn inline_list<I>(context: &Context, items: I) -> String
where
    I: IntoIterator,
    I::Item: Display,
{
    let mut items = items.into_iter();
    let shown = if context.full_lists { usize::MAX } else { 60 };
    let mut list = items
        .by_ref()
        .take(shown)
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let remainder = items.count();
    if remainder > 0 {
        list.push_str(&format!(" … and {remainder} more"));
    }
    list
}

/// Formats rows as a Markdown table.
///
/// Pipes inside cells are replaced with U+2223 (∣), since the terminal
/// renderer splits cells on every pipe. Unless `context.full_lists` is set,
/// only the first 20 rows are shown, followed by a count of the rest.
pub fn markdown_table<I, R>(context: &Context, headers: &[&str], rows: I) -> String
where
    I: IntoIterator<Item = R>,
    R: IntoIterator,
    R::Item: Display,
{
    let cell = |s: String| s.replace('|', "∣").replace('\n', " ");
    let line = |cells: Vec<String>| format!("| {} |", cells.join(" | "));
    let mut out = vec![
        line(headers.iter().map(|h| cell(h.to_string())).collect()),
        line(headers.iter().map(|_| "---".to_string()).collect()),
    ];
    let mut rows = rows.into_iter();
    let shown = if context.full_lists { usize::MAX } else { 20 };
    out.extend(
        rows.by_ref()
            .take(shown)
            .map(|r| line(r.into_iter().map(|c| cell(c.to_string())).collect())),
    );
    let remainder = rows.count();
    let mut table = out.join("\n");
    if remainder > 0 {
        table.push_str(&format!("\n\n... and {remainder} more rows"));
    }
    table
}

/// Asserts that all the values in a list are the same.
///
/// Each value is passed as a tuple of three values:
/// * The element to compare
/// * A displayable value to include in the message
/// * A label for the value to include in the message
///
/// For example:
///
/// ```rust,ignore
///     &[
///      (0b00000001, "Italic", "FontA.ttf"),
///      (0b00000001, "Italic", "FontB.ttf"),
///      (0b00100000, "Bold",   "FontC.ttf"),
///     ]
/// ```
///
/// The values are compared for equality.
/// If they are not equal, a failure status is returned, with a message listing all the values.
/// If they are, a pass status is returned.
pub fn assert_all_the_same<T, U, V>(
    _context: &Context,
    values: &[(T, U, V)],
    code: &str,
    message_start: &str,
    severity: StatusCode,
) -> CheckFnResult
where
    T: Eq,
    U: Display,
    V: Display,
{
    #[allow(clippy::indexing_slicing)] // If we are inside .all, then there must be a first element
    let ok = values.iter().all(|(a, _, _)| a == &values[0].0);
    if ok {
        Ok(Status::just_one_pass())
    } else {
        // One row per distinct value, listing the files that have it, most common first.
        let mut groups: Vec<(&T, String, Vec<String>)> = vec![];
        for (key, shown, label) in values {
            let label = label.to_string();
            let label = label.rsplit('/').next().unwrap_or(&label).to_string();
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.2.push(label),
                None => groups.push((key, shown.to_string(), vec![label])),
            }
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.2.len()));
        let message = format!(
            "{}\n\n{}",
            message_start,
            markdown_table(
                _context,
                &["Value", "Fonts", "Files"],
                groups.iter().map(|(_, shown, files)| vec![
                    format!("`{shown}`"),
                    files.len().to_string(),
                    files.join(", ")
                ])
            )
        );
        if StatusCode::Fail == severity {
            Ok(Status::just_one_fail(code, &message))
        } else {
            Ok(Status::just_one_warn(code, &message))
        }
    }
}
