use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// One recorded invocation: which verb ran what, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEntry {
    pub verb: String,
    pub command: String,
    pub args: Vec<String>,
    pub timestamp_unix_millis: u128,
}

impl UsageEntry {
    pub fn now(verb: &str, command: &str, args: &[String]) -> Self {
        let timestamp_unix_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self {
            verb: verb.to_string(),
            command: command.to_string(),
            args: args.to_vec(),
            timestamp_unix_millis,
        }
    }

    fn to_line(&self) -> String {
        let mut fields = vec![
            self.timestamp_unix_millis.to_string(),
            escape_field(&self.verb),
            escape_field(&self.command),
        ];
        fields.extend(self.args.iter().map(|arg| escape_field(arg)));
        fields.join("\t")
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

pub trait HistoryStore {
    fn record(&self, entry: &UsageEntry) -> std::io::Result<()>;
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
}
