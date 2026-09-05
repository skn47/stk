use std::collections::BTreeMap;
use std::io::Write;

use crate::history::{HistoryStore, UsageEntry};

struct Options {
    project_scope: bool,
    show_history: bool,
    show_graph: bool,
    show_quota: bool,
    tier: String,
    daily: bool,
    weekly: bool,
    monthly: bool,
    format: String,
    failures: bool,
    reset: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            project_scope: false,
            show_history: false,
            show_graph: false,
            show_quota: false,
            tier: "20x".to_string(),
            daily: false,
            weekly: false,
            monthly: false,
            format: "text".to_string(),
            failures: false,
            reset: false,
        }
    }
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-p" | "--project" => options.project_scope = true,
            "-H" | "--history" => options.show_history = true,
            "-g" | "--graph" => options.show_graph = true,
            "-q" | "--quota" => options.show_quota = true,
            "-d" | "--daily" => options.daily = true,
            "-w" | "--weekly" => options.weekly = true,
            "-m" | "--monthly" => options.monthly = true,
            "-a" | "--all" => {
                options.daily = true;
                options.weekly = true;
                options.monthly = true;
            }
            "-F" | "--failures" => options.failures = true,
            "--reset" => options.reset = true,
            "-t" | "--tier" => {
                options.tier = iter
                    .next()
                    .ok_or_else(|| "-t/--tier requires a value".to_string())?
                    .clone();
            }
            "-f" | "--format" => {
                options.format = iter
                    .next()
                    .ok_or_else(|| "-f/--format requires a value".to_string())?
                    .clone();
            }
            other => return Err(format!("unrecognized argument '{other}'")),
        }
    }
    Ok(options)
}

/// `stk gain`: shows a token-savings summary/history, matching the documented subset of
/// `rtk gain`'s flags. Only entries with recorded token counts count toward it -- plain
/// usage-tracking entries (e.g. from `stk proxy`) have nothing to summarize.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    history: &dyn HistoryStore,
) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(err) => {
            let _ = writeln!(stderr, "stk: gain: {err}");
            return 2;
        }
    };

    if options.reset {
        return match history.reset() {
            Ok(()) => {
                let _ = writeln!(stdout, "Savings history reset.");
                0
            }
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to reset history: {err}");
                1
            }
        };
    }

    if options.failures {
        let _ = writeln!(
            stdout,
            "No parse failures recorded.\n\
             STK's Specialists don't have a raw-execution fallback path this release."
        );
        return 0;
    }

    let all_entries = match history.read_all() {
        Ok(entries) => entries,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to read history: {err}");
            return 1;
        }
    };

    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string));
    let entries: Vec<&UsageEntry> = all_entries
        .iter()
        .filter(|e| e.input_tokens.is_some() && e.output_tokens.is_some())
        .filter(|e| !options.project_scope || e.cwd == cwd)
        .collect();

    if entries.is_empty() {
        let _ = writeln!(
            stdout,
            "No tracking data yet.\nRun some stk commands to start tracking savings."
        );
        return 0;
    }

    render(&entries, &options, cwd.as_deref(), stdout);
    0
}

struct Summary {
    total_commands: usize,
    total_input: usize,
    total_output: usize,
    total_saved: usize,
    avg_savings_pct: f64,
}

fn summarize(entries: &[&UsageEntry]) -> Summary {
    let total_commands = entries.len();
    let total_input: usize = entries.iter().filter_map(|e| e.input_tokens).sum();
    let total_output: usize = entries.iter().filter_map(|e| e.output_tokens).sum();
    let total_saved = total_input.saturating_sub(total_output);
    let avg_savings_pct = if total_input > 0 {
        total_saved as f64 / total_input as f64 * 100.0
    } else {
        0.0
    };
    Summary {
        total_commands,
        total_input,
        total_output,
        total_saved,
        avg_savings_pct,
    }
}

fn display_name(entry: &UsageEntry) -> String {
    if entry.command.is_empty() {
        entry.verb.clone()
    } else {
        match entry.args.first() {
            Some(first_arg) => format!("{} {first_arg}", entry.command),
            None => entry.command.clone(),
        }
    }
}

fn render(entries: &[&UsageEntry], options: &Options, cwd: Option<&str>, stdout: &mut dyn Write) {
    match options.format.as_str() {
        "json" => render_json(entries, stdout),
        "csv" => render_csv(entries, stdout),
        _ => render_text(entries, options, cwd, stdout),
    }
}

fn render_json(entries: &[&UsageEntry], stdout: &mut dyn Write) {
    let s = summarize(entries);
    let _ = writeln!(
        stdout,
        "{{\n  \"total_commands\": {},\n  \"total_input\": {},\n  \"total_output\": {},\n  \"total_saved\": {},\n  \"avg_savings_pct\": {:.4}\n}}",
        s.total_commands, s.total_input, s.total_output, s.total_saved, s.avg_savings_pct
    );
}

fn render_csv(entries: &[&UsageEntry], stdout: &mut dyn Write) {
    let _ = writeln!(stdout, "command,count,input,output,saved,pct");
    for (name, group) in by_command(entries) {
        let s = summarize(&group);
        let _ = writeln!(
            stdout,
            "{},{},{},{},{},{:.1}",
            csv_field(&name),
            s.total_commands,
            s.total_input,
            s.total_output,
            s.total_saved,
            s.avg_savings_pct
        );
    }
}

/// Quotes a CSV field (doubling any embedded quotes) whenever it contains a comma,
/// quote, or newline -- `name` comes from recorded command/argument text, which can
/// contain any of those.
fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn by_command<'a>(entries: &[&'a UsageEntry]) -> Vec<(String, Vec<&'a UsageEntry>)> {
    let mut grouped: BTreeMap<String, Vec<&UsageEntry>> = BTreeMap::new();
    for &entry in entries {
        grouped.entry(display_name(entry)).or_default().push(entry);
    }
    grouped.into_iter().collect()
}

fn render_text(
    entries: &[&UsageEntry],
    options: &Options,
    cwd: Option<&str>,
    stdout: &mut dyn Write,
) {
    let s = summarize(entries);
    if options.project_scope {
        let _ = writeln!(stdout, "STK Token Savings (Project Scope)");
        let _ = writeln!(stdout, "Scope: {}", cwd.unwrap_or("?"));
    } else {
        let _ = writeln!(stdout, "STK Token Savings (Global Scope)");
    }
    let _ = writeln!(stdout, "{}", "=".repeat(50));
    let _ = writeln!(stdout);
    let _ = writeln!(stdout, "Total commands:  {}", s.total_commands);
    let _ = writeln!(stdout, "Input tokens:    {}", s.total_input);
    let _ = writeln!(stdout, "Output tokens:   {}", s.total_output);
    let _ = writeln!(
        stdout,
        "Tokens saved:    {} ({:.1}%)",
        s.total_saved, s.avg_savings_pct
    );

    let _ = writeln!(stdout, "\nBy Command");
    let _ = writeln!(stdout, "{}", "-".repeat(60));
    for (name, group) in by_command(entries) {
        let cs = summarize(&group);
        let _ = writeln!(
            stdout,
            "  {name:<30} {:>5}  {:>6} ({:.1}%)",
            cs.total_commands, cs.total_saved, cs.avg_savings_pct
        );
    }

    if options.show_history {
        let _ = writeln!(stdout, "\nRecent Commands");
        let _ = writeln!(stdout, "{}", "-".repeat(60));
        let mut recent: Vec<&&UsageEntry> = entries.iter().collect();
        recent.sort_by_key(|e| std::cmp::Reverse(e.timestamp_unix_millis));
        for entry in recent.into_iter().take(20) {
            let pct = savings_pct(entry);
            let _ = writeln!(
                stdout,
                "  {} {:<30} {pct:.0}%",
                entry.timestamp_unix_millis / 1000,
                display_name(entry)
            );
        }
    }

    if options.daily {
        render_breakdown(entries, stdout, "Daily", daily_key);
    }
    if options.weekly {
        render_breakdown(entries, stdout, "Weekly", weekly_key);
    }
    if options.monthly {
        render_breakdown(entries, stdout, "Monthly", monthly_key);
    }

    if options.show_graph {
        let _ = writeln!(stdout, "\nDaily Savings");
        let _ = writeln!(stdout, "{}", "-".repeat(60));
        let mut by_day: BTreeMap<String, usize> = BTreeMap::new();
        for &entry in entries {
            let saved = entry
                .input_tokens
                .unwrap_or(0)
                .saturating_sub(entry.output_tokens.unwrap_or(0));
            *by_day
                .entry(daily_key(entry.timestamp_unix_millis))
                .or_insert(0) += saved;
        }
        let max = by_day.values().copied().max().unwrap_or(1).max(1);
        for (day, saved) in by_day {
            let bar_len = (saved * 40 / max).max(if saved > 0 { 1 } else { 0 });
            let _ = writeln!(stdout, "  {day} {} {saved}", "#".repeat(bar_len));
        }
    }

    if options.show_quota {
        let baseline = match options.tier.as_str() {
            "pro" => 6_000_000usize,
            "5x" => 30_000_000usize,
            _ => 120_000_000usize, // 20x, and the default for an unrecognized tier
        };
        let preserved_pct = if baseline > 0 {
            s.total_saved as f64 / baseline as f64 * 100.0
        } else {
            0.0
        };
        let _ = writeln!(stdout, "\nMonthly Quota Analysis");
        let _ = writeln!(stdout, "{}", "-".repeat(60));
        let _ = writeln!(stdout, "Subscription tier: {}", options.tier);
        let _ = writeln!(stdout, "Estimated monthly quota: ~{baseline}");
        let _ = writeln!(stdout, "Tokens saved (lifetime): {}", s.total_saved);
        let _ = writeln!(stdout, "Quota preserved: {preserved_pct:.4}%");
        let _ = writeln!(
            stdout,
            "\nNote: heuristic estimate, not a guarantee against any real subscription quota."
        );
    }
}

fn savings_pct(entry: &UsageEntry) -> f64 {
    let input = entry.input_tokens.unwrap_or(0);
    let output = entry.output_tokens.unwrap_or(0);
    if input == 0 {
        0.0
    } else {
        input.saturating_sub(output) as f64 / input as f64 * 100.0
    }
}

fn render_breakdown(
    entries: &[&UsageEntry],
    stdout: &mut dyn Write,
    label: &str,
    key: impl Fn(u128) -> String,
) {
    let mut grouped: BTreeMap<String, Vec<&UsageEntry>> = BTreeMap::new();
    for &entry in entries {
        grouped
            .entry(key(entry.timestamp_unix_millis))
            .or_default()
            .push(entry);
    }
    let _ = writeln!(stdout, "\n{label} Breakdown");
    let _ = writeln!(stdout, "{}", "-".repeat(60));
    for (bucket, group) in grouped {
        let s = summarize(&group);
        let _ = writeln!(
            stdout,
            "  {bucket:<20} {:>5} cmds  {:>6} saved  {:.1}%",
            s.total_commands, s.total_saved, s.avg_savings_pct
        );
    }
}

fn days_since_epoch(timestamp_unix_millis: u128) -> i64 {
    (timestamp_unix_millis / 86_400_000) as i64
}

/// Howard Hinnant's `civil_from_days`: converts a day count since the Unix epoch (UTC)
/// into a `(year, month, day)` civil date, without needing a date/time dependency.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

fn daily_key(timestamp_unix_millis: u128) -> String {
    let (y, m, d) = civil_from_days(days_since_epoch(timestamp_unix_millis));
    format!("{y:04}-{m:02}-{d:02}")
}

fn monthly_key(timestamp_unix_millis: u128) -> String {
    let (y, m, _) = civil_from_days(days_since_epoch(timestamp_unix_millis));
    format!("{y:04}-{m:02}")
}

fn weekly_key(timestamp_unix_millis: u128) -> String {
    let z = days_since_epoch(timestamp_unix_millis);
    // 1970-01-01 (z=0) was a Thursday; Monday=0..Sunday=6 for the week-start offset.
    let weekday0 = (z + 3).rem_euclid(7);
    let week_start = z - weekday0;
    let (_, sm, sd) = civil_from_days(week_start);
    let (_, em, ed) = civil_from_days(week_start + 6);
    format!("{sm:02}-{sd:02} to {em:02}-{ed:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19), (1970, 1, 20));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn daily_key_formats_as_iso_date() {
        // 1970-01-02 00:00:00 UTC
        assert_eq!(daily_key(86_400_000), "1970-01-02");
    }

    #[test]
    fn weekly_key_spans_seven_days_starting_monday() {
        // 1970-01-01 was a Thursday -- its week runs Dec 29 (Mon) to Jan 4 (Sun).
        assert_eq!(weekly_key(0), "12-29 to 01-04");
    }

    use crate::history::FakeHistoryStore;

    fn run(args: &[&str], history: &FakeHistoryStore) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let exit_code = dispatch(&args, &mut stdout, &mut stderr, history);
        (
            exit_code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    fn tracked_entry(command: &str, input: usize, output: usize) -> UsageEntry {
        UsageEntry::now_with_tokens(
            "compile",
            command,
            &[],
            Some(input),
            Some(output),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(str::to_string)),
        )
    }

    #[test]
    fn no_data_prints_a_clear_message_not_a_zeroed_summary() {
        let history = FakeHistoryStore::new();
        let (code, stdout, _) = run(&[], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("No tracking data yet"));
    }

    #[test]
    fn entries_with_no_token_counts_are_excluded_from_the_summary() {
        let history = FakeHistoryStore::new();
        history
            .record(&UsageEntry::now("proxy", "echo", &[]))
            .unwrap();
        let (code, stdout, _) = run(&[], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("No tracking data yet"));
    }

    #[test]
    fn a_plain_summary_reports_totals_and_reduction() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        let (code, stdout, _) = run(&[], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("Input tokens:    100"));
        assert!(stdout.contains("Output tokens:   40"));
        assert!(stdout.contains("Tokens saved:    60"));
    }

    #[test]
    fn format_json_emits_the_documented_summary_fields() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        let (code, stdout, _) = run(&["-f", "json"], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("\"total_input\": 100"));
        assert!(stdout.contains("\"total_output\": 40"));
    }

    #[test]
    fn format_csv_emits_a_header_and_one_row_per_command() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        history.record(&tracked_entry("git", 50, 45)).unwrap();
        let (code, stdout, _) = run(&["-f", "csv"], &history);
        assert_eq!(code, 0);
        assert_eq!(
            stdout.lines().count(),
            3,
            "header + 2 command rows, got: {stdout:?}"
        );
        assert!(stdout.contains("cargo,1,100,40,60"));
        assert!(stdout.contains("git,1,50,45,5"));
    }

    #[test]
    fn format_csv_quotes_a_command_containing_a_comma() {
        let history = FakeHistoryStore::new();
        history
            .record(&tracked_entry("build, release", 100, 40))
            .unwrap();
        let (code, stdout, _) = run(&["-f", "csv"], &history);
        assert_eq!(code, 0);
        assert!(
            stdout.contains("\"build, release\",1,100,40,60"),
            "got: {stdout:?}"
        );
    }

    #[test]
    fn reset_clears_history_so_a_later_gain_reports_no_data() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        let (code, stdout, _) = run(&["--reset"], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("reset"));
        let (_, stdout, _) = run(&[], &history);
        assert!(stdout.contains("No tracking data yet"));
    }

    #[test]
    fn failures_flag_shows_a_standalone_message_not_the_summary() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        let (code, stdout, _) = run(&["-F"], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("No parse failures"));
        assert!(!stdout.contains("Input tokens"));
    }

    #[test]
    fn project_scope_filters_to_entries_recorded_from_the_current_directory() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        history
            .record(&UsageEntry::now_with_tokens(
                "compile",
                "git",
                &[],
                Some(10),
                Some(5),
                Some("/some/other/project".to_string()),
            ))
            .unwrap();
        let (code, stdout, _) = run(&["-p"], &history);
        assert_eq!(code, 0);
        assert!(stdout.contains("Project Scope"));
        assert!(stdout.contains("Total commands:  1"));
    }

    #[test]
    fn an_unrecognized_argument_is_an_error() {
        let history = FakeHistoryStore::new();
        let (code, _, stderr) = run(&["--bogus"], &history);
        assert_ne!(code, 0);
        assert!(stderr.contains("unrecognized argument"));
    }

    #[test]
    fn daily_and_history_flags_add_their_own_sections() {
        let history = FakeHistoryStore::new();
        history.record(&tracked_entry("cargo", 100, 40)).unwrap();
        let (_, stdout, _) = run(&["-d", "-H"], &history);
        assert!(stdout.contains("Daily Breakdown"));
        assert!(stdout.contains("Recent Commands"));
    }
}
