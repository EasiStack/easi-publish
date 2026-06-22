//! Small shared helpers for the response wrappers.

/// Sanitize a user-supplied download filename for use in a `Content-Disposition`
/// header: keep alphanumerics plus `-_.`, replace everything else with `_`.
pub(crate) fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}
