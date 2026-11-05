use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Config {
    pub db_folder: String,
    pub apprise_url: String,
    pub apprise_tag_prefix: String,
    pub dev_username: String,
    pub dev_name: String,
    pub port: u16,
    pub profile: String,
    pub config_file: String,
    pub mcp_keys: HashMap<String, String>,
}

impl Config {
    pub fn from_env() -> Self {
        let db_folder = env::var("DB_FOLDER").unwrap_or_else(|_| ".".to_string());
        let apprise_url = env::var("APPRISE_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:8000/notify".to_string());
        let apprise_tag_prefix =
            env::var("APPRISE_TAG_PREFIX").unwrap_or_else(|_| "reminders".to_string());
        let dev_username = env::var("DEV_USERNAME").unwrap_or_else(|_| "dev_user".to_string());
        let dev_name = env::var("DEV_NAME").unwrap_or_else(|_| "Dev User".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);
        let profile = env::var("APP_PROFILE").unwrap_or_else(|_| "prod".to_string());
        let config_file =
            env::var("CONFIG_FILE").unwrap_or_else(|_| "/data/config.yaml".to_string());

        let mcp_keys = load_mcp_keys_from_file(&config_file);

        Config {
            db_folder,
            apprise_url,
            apprise_tag_prefix,
            dev_username,
            dev_name,
            port,
            profile,
            config_file,
            mcp_keys,
        }
    }

    pub fn get_mcp_keys(&self) -> HashMap<String, String> {
        if Path::new(&self.config_file).exists() {
            let loaded = load_mcp_keys_from_file(&self.config_file);
            if !loaded.is_empty() || self.mcp_keys.is_empty() {
                return loaded;
            }
        }
        self.mcp_keys.clone()
    }
}

pub fn load_mcp_keys_from_file(path: &str) -> HashMap<String, String> {
    match fs::read_to_string(path) {
        Ok(content) => {
            let keys = parse_mcp_keys(&content);
            tracing::debug!("Loaded {} MCP keys from {}", keys.len(), path);
            keys
        }
        Err(e) => {
            tracing::debug!("Could not read config file at {}: {}", path, e);
            HashMap::new()
        }
    }
}

pub fn parse_mcp_keys(yaml_content: &str) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    let mut in_mcp_keys = false;

    for raw_line in yaml_content.lines() {
        let line_without_comment = match raw_line.find('#') {
            Some(idx) => &raw_line[..idx],
            None => raw_line,
        };
        let trimmed = line_without_comment.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_indented_or_list =
            raw_line.starts_with(' ') || raw_line.starts_with('\t') || trimmed.starts_with('-');

        if !is_indented_or_list {
            if let Some(rest) = trimmed
                .strip_prefix("mcp-keys:")
                .or_else(|| trimmed.strip_prefix("mcp_keys:"))
            {
                in_mcp_keys = true;
                let rest = rest.trim();
                if !rest.is_empty() && rest != "[]" && rest != "{}" {
                    parse_key_entry(rest, &mut keys);
                }
            } else {
                in_mcp_keys = false;
            }
            continue;
        }

        if in_mcp_keys {
            let item = trimmed
                .strip_prefix('-')
                .map(|s| s.trim())
                .unwrap_or(trimmed);
            parse_key_entry(item, &mut keys);
        }
    }

    keys
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_key_entry(entry: &str, keys: &mut HashMap<String, String>) {
    let unquoted = strip_quotes(entry);
    if let Some((k, v)) = unquoted.split_once('=') {
        let user = strip_quotes(k).trim();
        let secret = strip_quotes(v).trim();
        if !user.is_empty() && !secret.is_empty() {
            keys.insert(user.to_string(), secret.to_string());
        }
    } else if let Some((k, v)) = unquoted.split_once(':') {
        let user = strip_quotes(k).trim();
        let secret = strip_quotes(v).trim();
        if !user.is_empty() && !secret.is_empty() {
            keys.insert(user.to_string(), secret.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mcp_keys_hyphenated() {
        let yaml = r#"
mcp-keys:
  - dudu=someSuperSecretKey
  - alice=anotherSecret=WithEquals
"#;
        let keys = parse_mcp_keys(yaml);
        assert_eq!(
            keys.get("dudu").map(String::as_str),
            Some("someSuperSecretKey")
        );
        assert_eq!(
            keys.get("alice").map(String::as_str),
            Some("anotherSecret=WithEquals")
        );
    }

    #[test]
    fn test_parse_mcp_keys_underscored_and_comments() {
        let yaml = r#"
# Main configuration
mcp_keys:
  - "bob=secret123" # inline comment
  - 'carol=secret456'
other_section:
  - ignored=value
"#;
        let keys = parse_mcp_keys(yaml);
        assert_eq!(keys.len(), 2);
        assert_eq!(keys.get("bob").map(String::as_str), Some("secret123"));
        assert_eq!(keys.get("carol").map(String::as_str), Some("secret456"));
    }
}
