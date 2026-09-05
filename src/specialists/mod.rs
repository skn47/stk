pub mod cargo;

use crate::chunk::Classifier;

/// The classifier registered for `command`, or `None` if no Specialist handles it --
/// callers must error as "unsupported" rather than silently falling back to generic.
pub fn lookup(command: &str) -> Option<Classifier> {
    match command {
        "cargo" => Some(cargo::classify),
        _ => None,
    }
}
