/// The fully-resolved config: every field concrete, after precedence (`CLI flags > env
/// vars > project config > global config > built-in defaults`) is applied.
/// `intent`/`session_memory`/`session_ttl_minutes` are forward-compatible schema only.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub budget: usize,
    pub intent: String,
    pub session_memory: bool,
    pub session_ttl_minutes: u32,
}

impl Settings {
    pub fn defaults() -> Self {
        Settings {
            budget: 2000,
            intent: "auto".to_string(),
            session_memory: false,
            session_ttl_minutes: 120,
        }
    }
}

/// One layer of the precedence chain, every field optional: `None` means "this layer
/// didn't set it," not "set it to a default." Also the seam env vars come in through --
/// production builds one from `std::env::var`, tests construct it by hand.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PartialSettings {
    pub budget: Option<usize>,
    pub intent: Option<String>,
    pub session_memory: Option<bool>,
    pub session_ttl_minutes: Option<u32>,
}

fn merge(base: PartialSettings, over: PartialSettings) -> PartialSettings {
    PartialSettings {
        budget: over.budget.or(base.budget),
        intent: over.intent.or(base.intent),
        session_memory: over.session_memory.or(base.session_memory),
        session_ttl_minutes: over.session_ttl_minutes.or(base.session_ttl_minutes),
    }
}

/// Reads an integer key, hard-erroring (not silently ignoring) if it's present but out
/// of range for `T` -- unlike a wrong-typed value, an out-of-range one can't be a
/// forward-compatible future key, only a mistake in this config file.
fn bounded_integer<T: TryFrom<i64>>(table: &toml::Table, key: &str) -> Result<Option<T>, String> {
    match table.get(key).and_then(|v| v.as_integer()) {
        None => Ok(None),
        Some(n) => T::try_from(n)
            .map(Some)
            .map_err(|_| format!("invalid config: '{key}' value {n} is out of range")),
    }
}

fn parse_toml_settings(text: &str) -> Result<PartialSettings, String> {
    let table: toml::Table =
        toml::from_str(text).map_err(|err| format!("invalid config: {err}"))?;
    Ok(PartialSettings {
        budget: bounded_integer(&table, "budget")?,
        intent: table
            .get("intent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        session_memory: table.get("session_memory").and_then(|v| v.as_bool()),
        session_ttl_minutes: bounded_integer(&table, "session_ttl_minutes")?,
    })
}

/// Resolves the effective config across all four precedence layers. `cli_budget` is the
/// only field a CLI flag can set this release -- `--intent` is a hard parse error.
pub fn resolve(
    cli_budget: Option<usize>,
    env: PartialSettings,
    project_toml: Option<&str>,
    global_toml: Option<&str>,
) -> Result<Settings, String> {
    let global = match global_toml {
        Some(text) => parse_toml_settings(text)?,
        None => PartialSettings::default(),
    };
    let project = match project_toml {
        Some(text) => parse_toml_settings(text)?,
        None => PartialSettings::default(),
    };
    let cli = PartialSettings {
        budget: cli_budget,
        ..Default::default()
    };

    let merged = merge(merge(merge(global, project), env), cli);
    let defaults = Settings::defaults();
    Ok(Settings {
        budget: merged.budget.unwrap_or(defaults.budget),
        intent: merged.intent.unwrap_or(defaults.intent),
        session_memory: merged.session_memory.unwrap_or(defaults.session_memory),
        session_ttl_minutes: merged
            .session_ttl_minutes
            .unwrap_or(defaults.session_ttl_minutes),
    })
}

/// Renders as TOML via the library's own serializer (not hand-formatted `format!`), so
/// values needing escaping (quotes, backslashes, control characters in `intent`) come
/// out as valid TOML rather than corrupting the file.
pub fn render(settings: &Settings) -> String {
    let mut table = toml::Table::new();
    table.insert(
        "budget".to_string(),
        toml::Value::Integer(settings.budget as i64),
    );
    table.insert(
        "intent".to_string(),
        toml::Value::String(settings.intent.clone()),
    );
    table.insert(
        "session_memory".to_string(),
        toml::Value::Boolean(settings.session_memory),
    );
    table.insert(
        "session_ttl_minutes".to_string(),
        toml::Value::Integer(settings.session_ttl_minutes as i64),
    );
    toml::to_string(&table).unwrap_or_default()
}

/// Atomically writes the default settings to `path` unless it already exists (an
/// existing file is left completely untouched). Shared by every caller that needs a
/// fresh, default project or global config file created idempotently.
pub fn create_default_file(path: &std::path::Path) -> std::io::Result<bool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = render(&Settings::defaults());
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(contents.as_bytes())?;
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_budget(budget: usize) -> PartialSettings {
        PartialSettings {
            budget: Some(budget),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_apply_when_nothing_is_set_anywhere() {
        let resolved = resolve(None, PartialSettings::default(), None, None).unwrap();
        assert_eq!(resolved, Settings::defaults());
    }

    #[test]
    fn global_config_overrides_built_in_defaults() {
        let resolved = resolve(
            None,
            PartialSettings::default(),
            None,
            Some("budget = 3000"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 3000);
    }

    #[test]
    fn project_config_overrides_global_config() {
        let resolved = resolve(
            None,
            PartialSettings::default(),
            Some("budget = 4000"),
            Some("budget = 3000"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 4000);
    }

    #[test]
    fn env_var_overrides_project_config() {
        let resolved = resolve(
            None,
            env_budget(5000),
            Some("budget = 4000"),
            Some("budget = 3000"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 5000);
    }

    #[test]
    fn cli_flag_overrides_env_var() {
        let resolved = resolve(
            Some(6000),
            env_budget(5000),
            Some("budget = 4000"),
            Some("budget = 3000"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 6000);
    }

    #[test]
    fn a_layer_missing_one_field_falls_through_to_the_layer_below() {
        let resolved = resolve(
            None,
            PartialSettings::default(),
            Some("intent = \"debug\""),
            Some("budget = 3000"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 3000);
        assert_eq!(resolved.intent, "debug");
    }

    #[test]
    fn intent_session_memory_and_ttl_are_parsed_but_have_no_behavioral_effect() {
        let resolved = resolve(
            None,
            PartialSettings::default(),
            Some("intent = \"debug\"\nsession_memory = true\nsession_ttl_minutes = 45"),
            None,
        )
        .unwrap();
        assert_eq!(resolved.intent, "debug");
        assert!(resolved.session_memory);
        assert_eq!(resolved.session_ttl_minutes, 45);
    }

    #[test]
    fn an_unrecognized_config_key_does_not_fail_parsing() {
        let resolved = resolve(
            None,
            PartialSettings::default(),
            None,
            Some("budget = 3000\n[tree_sitter]\nenabled = true"),
        )
        .unwrap();
        assert_eq!(resolved.budget, 3000);
    }

    #[test]
    fn malformed_toml_is_a_clear_error() {
        let err =
            resolve(None, PartialSettings::default(), Some("not = [valid"), None).unwrap_err();
        assert!(err.contains("invalid config"));
    }

    #[test]
    fn a_negative_budget_is_a_clear_error_not_a_silent_wraparound() {
        let err = resolve(None, PartialSettings::default(), None, Some("budget = -1")).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn render_escapes_special_characters_in_intent() {
        let mut settings = Settings::defaults();
        settings.intent = "say \"hi\"\\now".to_string();
        let rendered = render(&settings);
        let parsed: toml::Table = toml::from_str(&rendered).unwrap();
        assert_eq!(
            parsed.get("intent").and_then(|v| v.as_str()),
            Some("say \"hi\"\\now")
        );
    }
}
