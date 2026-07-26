use reqwest::header::HeaderValue;
use std::sync::{LazyLock, Mutex};

pub const DEFAULT_ORIGINATOR: &str = "ring_cli_rs";
pub const ORIGINATOR_OVERRIDE_ENV: &str = "RING_ORIGINATOR_OVERRIDE";

const CODEX_ORIGINATOR: &str = "codex_cli_rs";
const CODEX_VERSION: &str = "0.118.0";
const CLAUDE_CLI_VERSION: &str = "2.1.75";

static USER_AGENT_SUFFIX: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub fn set_user_agent_suffix(suffix: &str) {
    if let Ok(mut guard) = USER_AGENT_SUFFIX.lock() {
        *guard = Some(suffix.to_string());
    }
}

pub fn originator() -> String {
    std::env::var(ORIGINATOR_OVERRIDE_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_ORIGINATOR.to_string())
}

fn os_segment() -> String {
    let os = os_info::get();
    format!(
        "{} {}; {}",
        os.os_type(),
        os.version(),
        os.architecture().unwrap_or("unknown"),
    )
}

pub fn get_user_agent() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let orig = originator();

    let prefix = format!(
        "{}/{version} ({}) {}",
        orig,
        os_segment(),
        terminal_token(),
    );

    let suffix = USER_AGENT_SUFFIX
        .lock()
        .ok()
        .and_then(|guard| guard.clone());
    let suffix = suffix
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map_or_else(String::new, |v| format!(" ({v})"));

    let candidate = format!("{prefix}{suffix}");
    sanitize_user_agent(candidate, &prefix)
}

pub fn codex_user_agent() -> String {
    let prefix = format!(
        "{CODEX_ORIGINATOR}/{CODEX_VERSION} ({}) {}",
        os_segment(),
        terminal_token(),
    );
    sanitize_user_agent(prefix.clone(), &prefix)
}

pub fn claude_user_agent() -> String {
    format!("claude-cli/{CLAUDE_CLI_VERSION}")
}

pub fn default_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&originator()) {
        headers.insert("originator", val);
    }
    if let Ok(val) = HeaderValue::from_str(&get_user_agent()) {
        headers.insert(reqwest::header::USER_AGENT, val);
    }
    headers
}

pub fn codex_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("originator", HeaderValue::from_static(CODEX_ORIGINATOR));
    if let Ok(val) = HeaderValue::from_str(&codex_user_agent()) {
        headers.insert(reqwest::header::USER_AGENT, val);
    }
    headers
}

pub fn claude_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("originator", HeaderValue::from_static("claude_cli_rs"));
    headers.insert(
        reqwest::header::USER_AGENT,
        HeaderValue::from_static("claude-cli/2.1.75"),
    );
    headers
}

fn sanitize_user_agent(candidate: String, fallback: &str) -> String {
    if HeaderValue::from_str(candidate.as_str()).is_ok() {
        return candidate;
    }
    let sanitized: String = candidate
        .chars()
        .map(|ch| if matches!(ch, ' '..='~') { ch } else { '_' })
        .collect();
    if !sanitized.is_empty() && HeaderValue::from_str(sanitized.as_str()).is_ok() {
        tracing::warn!("sanitized ring user agent due to invalid header characters");
        sanitized
    } else if HeaderValue::from_str(fallback).is_ok() {
        tracing::warn!("falling back to base ring user agent");
        fallback.to_string()
    } else {
        tracing::warn!("falling back to default ring originator as user agent");
        DEFAULT_ORIGINATOR.to_string()
    }
}

fn terminal_token() -> String {
    if let Some(program) = env_non_empty("TERM_PROGRAM") {
        return match env_non_empty("TERM_PROGRAM_VERSION") {
            Some(ver) => sanitize_token(format!("{program}/{ver}")),
            None => sanitize_token(program),
        };
    }
    if let Some(ver) = env_non_empty("WEZTERM_VERSION") {
        return sanitize_token(format!("WezTerm/{ver}"));
    }
    if env_non_empty("ITERM_SESSION_ID").is_some() {
        return "iTerm.app".to_string();
    }
    if env_non_empty("KITTY_WINDOW_ID").is_some() {
        return "kitty".to_string();
    }
    if env_non_empty("ALACRITTY_LOG").is_some() {
        return "Alacritty".to_string();
    }
    if env_non_empty("KONSOLE_VERSION").is_some() {
        return sanitize_token(format!("Konsole/{}", env_non_empty("KONSOLE_VERSION").unwrap()));
    }
    if env_non_empty("WT_SESSION").is_some() {
        return "WindowsTerminal".to_string();
    }
    if let Some(term) = env_non_empty("TERM") {
        return sanitize_token(term);
    }
    "unknown".to_string()
}

fn sanitize_token(raw: String) -> String {
    raw.chars()
        .map(|ch| if matches!(ch, ' '..='~') { ch } else { '_' })
        .collect()
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}
