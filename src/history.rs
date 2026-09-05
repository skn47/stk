use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::budget::tokenizer::TokenCounter;

/// One recorded invocation: which verb ran what, when, and (for verbs that compress
/// something) how many tokens it saw going in and rendered going out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEntry {
    pub verb: String,
    pub command: String,
    pub args: Vec<String>,
    pub timestamp_unix_millis: u128,
    pub input_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
    pub cwd: Option<String>,
}

impl UsageEntry {
    pub fn now(verb: &str, command: &str, args: &[String]) -> Self {
        Self::now_with_tokens(verb, command, args, None, None, None)
    }

    pub fn now_with_tokens(
        verb: &str,
        command: &str,
        args: &[String],
        input_tokens: Option<usize>,
        output_tokens: Option<usize>,
        cwd: Option<String>,
    ) -> Self {
        let timestamp_unix_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self {
            verb: verb.to_string(),
            command: command.to_string(),
            args: args.to_vec(),
            timestamp_unix_millis,
            input_tokens,
            output_tokens,
            cwd,
        }
    }

    fn to_line(&self) -> String {
        let mut fields = vec![
            self.timestamp_unix_millis.to_string(),
            opt_usize_field(self.input_tokens),
            opt_usize_field(self.output_tokens),
            opt_str_field(self.cwd.as_deref()),
            escape_field(&self.verb),
            escape_field(&self.command),
        ];
        fields.extend(self.args.iter().map(|arg| escape_field(arg)));
        fields.join("\t")
    }

    /// Inverse of `to_line`. Returns `None` for a line that can't be parsed (a partial
    /// write from a crash mid-append, say) rather than erroring the whole read -- one
    /// bad line shouldn't make the rest of the history unreadable.
    fn from_line(line: &str) -> Option<UsageEntry> {
        let mut fields = line.split('\t');
        let timestamp_unix_millis = fields.next()?.parse().ok()?;
        let input_tokens = parse_opt_usize_field(fields.next()?);
        let output_tokens = parse_opt_usize_field(fields.next()?);
        let cwd = parse_opt_str_field(fields.next()?);
        let verb = unescape_field(fields.next()?);
        let command = unescape_field(fields.next()?);
        let args = fields.map(unescape_field).collect();
        Some(UsageEntry {
            verb,
            command,
            args,
            timestamp_unix_millis,
            input_tokens,
            output_tokens,
            cwd,
        })
    }
}

const NONE_SENTINEL: &str = "-";

fn opt_usize_field(value: Option<usize>) -> String {
    match value {
        Some(n) => n.to_string(),
        None => NONE_SENTINEL.to_string(),
    }
}

fn parse_opt_usize_field(field: &str) -> Option<usize> {
    if field == NONE_SENTINEL {
        None
    } else {
        field.parse().ok()
    }
}

fn opt_str_field(value: Option<&str>) -> String {
    match value {
        Some(s) => escape_field(s),
        None => NONE_SENTINEL.to_string(),
    }
}

fn parse_opt_str_field(field: &str) -> Option<String> {
    if field == NONE_SENTINEL {
        None
    } else {
        Some(unescape_field(field))
    }
}

/// Escapes backslash/tab/newline/carriage-return so a field can never be confused with
/// the tab field-separator or split a logical entry across lines.
fn escape_field(field: &str) -> String {
    field
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_field(field: &str) -> String {
    let mut result = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => result.push('\\'),
            Some('t') => result.push('\t'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

pub trait HistoryStore {
    fn record(&self, entry: &UsageEntry) -> std::io::Result<()>;
    /// All recorded entries, oldest first. A store with nothing recorded yet returns an
    /// empty `Vec`, not an error.
    fn read_all(&self) -> std::io::Result<Vec<UsageEntry>>;
    /// Clears every recorded entry.
    fn reset(&self) -> std::io::Result<()>;
}

/// Appends usage entries to a local history file, one tab-separated line per entry.
pub struct FileHistoryStore {
    path: PathBuf,
}

impl FileHistoryStore {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        cache_dir().join("stk").join("history.log")
    }
}

fn cache_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(xdg);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache");
    }
    // Neither is set (e.g. a minimal container or cron job): std::env::temp_dir() is
    // always an absolute, stable path, unlike falling back to a cwd-relative ".cache".
    std::env::temp_dir().join("stk-cache")
}

impl HistoryStore for FileHistoryStore {
    fn record(&self, entry: &UsageEntry) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        // One write_all call, not writeln!: O_APPEND only guarantees atomicity per
        // syscall, and writeln! would emit the content and the trailing newline as two.
        let mut line = entry.to_line();
        line.push('\n');
        file.write_all(line.as_bytes())
    }

    fn read_all(&self) -> std::io::Result<Vec<UsageEntry>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        Ok(text.lines().filter_map(UsageEntry::from_line).collect())
    }

    fn reset(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

/// Test double for [`HistoryStore`]: records entries in memory instead of writing to disk.
pub struct FakeHistoryStore {
    entries: Mutex<Vec<UsageEntry>>,
}

impl FakeHistoryStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn entries(&self) -> Vec<UsageEntry> {
        self.entries.lock().unwrap().clone()
    }
}

impl Default for FakeHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryStore for FakeHistoryStore {
    fn record(&self, entry: &UsageEntry) -> std::io::Result<()> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }

    fn read_all(&self) -> std::io::Result<Vec<UsageEntry>> {
        Ok(self.entries())
    }

    fn reset(&self) -> std::io::Result<()> {
        self.entries.lock().unwrap().clear();
        Ok(())
    }
}

/// Counts `input`/`output` and records the result against `history`. The one place every
/// compressing verb (`compile`, the `Specialist`s, the filters) reports its savings, so
/// `stk gain` has real data regardless of which verb produced it.
pub fn record_savings(
    history: &dyn HistoryStore,
    counter: &dyn TokenCounter,
    verb: &str,
    command: &str,
    args: &[String],
    input: &str,
    output: &str,
) {
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string));
    let entry = UsageEntry::now_with_tokens(
        verb,
        command,
        args,
        Some(counter.count(input)),
        Some(counter.count(output)),
        cwd,
    );
    let _ = history.record(&entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::tokenizer::ApproximateCounter;

    #[test]
    fn a_line_round_trips_through_to_line_and_from_line() {
        let entry = UsageEntry::now_with_tokens(
            "compile",
            "cargo",
            &["check".to_string(), "--all".to_string()],
            Some(120),
            Some(40),
            Some("/home/h/stk".to_string()),
        );
        let parsed = UsageEntry::from_line(&entry.to_line()).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn fields_containing_tabs_and_newlines_round_trip_safely() {
        let entry = UsageEntry::now(
            "proxy",
            "echo",
            &["a\tb".to_string(), "line1\nline2".to_string()],
        );
        let parsed = UsageEntry::from_line(&entry.to_line()).unwrap();
        assert_eq!(parsed.args, entry.args);
    }

    #[test]
    fn entries_with_no_token_counts_or_cwd_round_trip_as_none() {
        let entry = UsageEntry::now("proxy", "echo", &["hi".to_string()]);
        let parsed = UsageEntry::from_line(&entry.to_line()).unwrap();
        assert_eq!(parsed.input_tokens, None);
        assert_eq!(parsed.output_tokens, None);
        assert_eq!(parsed.cwd, None);
    }

    #[test]
    fn a_malformed_line_is_skipped_not_a_hard_error() {
        assert_eq!(UsageEntry::from_line("not enough fields"), None);
    }

    #[test]
    fn record_savings_counts_and_stores_both_sides() {
        let history = FakeHistoryStore::new();
        record_savings(
            &history,
            &ApproximateCounter,
            "compile",
            "cargo",
            &["check".to_string()],
            "a very long raw input that should count as more tokens",
            "short",
        );
        let entries = history.entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].input_tokens.unwrap() > entries[0].output_tokens.unwrap());
    }

    #[test]
    fn fake_history_store_reset_clears_recorded_entries() {
        let history = FakeHistoryStore::new();
        history
            .record(&UsageEntry::now("proxy", "echo", &[]))
            .unwrap();
        assert_eq!(history.read_all().unwrap().len(), 1);
        history.reset().unwrap();
        assert_eq!(history.read_all().unwrap().len(), 0);
    }
}
