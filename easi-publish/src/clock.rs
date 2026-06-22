/// Clock source for the template's `datetime.today()`.
///
/// The default ([`SystemLocal`](Clock::SystemLocal)) uses the host's local
/// time. Use [`Fixed`](Clock::Fixed) (or [`Utc`](Clock::Utc)) for deterministic,
/// host-independent output.
///
/// Shared by both the PDF and HTML paths. It controls `datetime.today()`, which
/// is independent of the output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    /// Host local time using `chrono::Local::now()` (default).
    SystemLocal,
    /// UTC time based on `chrono::Utc::now()`.
    Utc,
    /// A fixed calendar date that is fully reproducible.
    Fixed {
        /// Year (e.g. 2026).
        year: i32,
        /// Month, 1–12.
        month: u8,
        /// Day, 1–31.
        day: u8,
    },
}
