use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Redactor {
    pub redact_usernames: bool,
    pub redact_ips: bool,
    pub redact_tokens: bool,
    pub custom_replacements: Vec<(String, String)>,
}

impl Default for Redactor {
    fn default() -> Self {
        Self {
            redact_usernames: true,
            redact_ips: true,
            redact_tokens: true,
            custom_replacements: Vec::new(),
        }
    }
}

impl Redactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_custom_replacement(
        mut self,
        find: impl Into<String>,
        replace: impl Into<String>,
    ) -> Self {
        self.custom_replacements.push((find.into(), replace.into()));
        self
    }

    pub fn redact_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        for (from, to) in &self.custom_replacements {
            result = result.replace(from, to);
        }

        if self.redact_usernames {
            result = redact_usernames_in_paths(&result);
        }

        if self.redact_tokens {
            result = redact_tokens_and_secrets(&result);
        }

        if self.redact_ips {
            result = redact_ipv4_addresses(&result);
        }

        result
    }

    pub fn redact_value(&self, val: &Value) -> Value {
        match val {
            Value::String(s) => Value::String(self.redact_text(s)),
            Value::Array(arr) => {
                let redacted: Vec<Value> = arr.iter().map(|item| self.redact_value(item)).collect();
                Value::Array(redacted)
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.redact_value(v));
                }
                Value::Object(new_map)
            }
            other => other.clone(),
        }
    }
}

/// Redacts username segments from paths such as:
/// - `C:\Users\JohnDoe\AppData` -> `C:\Users\[USERNAME]\AppData`
/// - `C:/Users/JohnDoe/AppData` -> `C:/Users/[USERNAME]/AppData`
/// - `/home/johndoe/project` -> `/home/[USERNAME]/project`
/// - `/Users/johndoe/Library` -> `/Users/[USERNAME]/Library`
pub fn redact_usernames_in_paths(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    while i < len {
        // Check Windows Users: e.g. "Users\" or "Users/" preceded by optional drive "C:\" or "\"
        // Also check "/home/" and "/Users/"
        let remaining: String = chars[i..].iter().collect();

        // 1. Check "/home/"
        if remaining.starts_with("/home/") {
            out.push_str("/home/");
            i += 6;
            // find next separator or delimiter
            let user_start = i;
            while i < len
                && chars[i] != '/'
                && chars[i] != '\\'
                && !chars[i].is_whitespace()
                && chars[i] != '"'
                && chars[i] != '\''
                && chars[i] != ':'
            {
                i += 1;
            }
            if i > user_start {
                out.push_str("[USERNAME]");
            }
            continue;
        }

        // 2. Check "/Users/"
        if remaining.starts_with("/Users/") || remaining.starts_with("/users/") {
            out.push_str(&remaining[..7]);
            i += 7;
            let user_start = i;
            while i < len
                && chars[i] != '/'
                && chars[i] != '\\'
                && !chars[i].is_whitespace()
                && chars[i] != '"'
                && chars[i] != '\''
                && chars[i] != ':'
            {
                i += 1;
            }
            if i > user_start {
                out.push_str("[USERNAME]");
            }
            continue;
        }

        // 3. Check Windows "[A-Za-z]:\Users\" or "[A-Za-z]:/Users/" or "\Users\" or "Users\"
        let is_win_users = if remaining.len() >= 6 {
            let prefix_lower = remaining[..6.min(remaining.len())].to_lowercase();
            prefix_lower.starts_with("users\\") || prefix_lower.starts_with("users/")
        } else {
            false
        };

        if is_win_users {
            out.push_str(&remaining[..6]); // "Users\" or "Users/"
            i += 6;
            let user_start = i;
            while i < len
                && chars[i] != '/'
                && chars[i] != '\\'
                && !chars[i].is_whitespace()
                && chars[i] != '"'
                && chars[i] != '\''
                && chars[i] != ';'
                && chars[i] != ':'
            {
                i += 1;
            }
            if i > user_start {
                out.push_str("[USERNAME]");
            }
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

pub fn redact_tokens_and_secrets(input: &str) -> String {
    let mut res = input.to_string();
    let secret_prefixes = [
        "token=",
        "Token=",
        "TOKEN=",
        "password=",
        "Password=",
        "Bearer ",
    ];

    for prefix in &secret_prefixes {
        let mut start_idx = 0;
        while let Some(pos) = res[start_idx..].find(prefix) {
            let actual_pos = start_idx + pos;
            let val_start = actual_pos + prefix.len();
            // find end of token: whitespace, quote, comma, ampersand, newline
            let end_offset = res[val_start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&' || c == ';')
                .unwrap_or(res[val_start..].len());

            let val_end = val_start + end_offset;
            if val_end > val_start {
                let before = &res[..val_start];
                let after = &res[val_end..];
                res = format!("{}[REDACTED]{}", before, after);
                start_idx = val_start + "[REDACTED]".len();
            } else {
                start_idx = val_start;
            }
        }
    }
    res
}

pub fn redact_ipv4_addresses(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for digit that might start IPv4
        if chars[i].is_ascii_digit()
            && (i == 0 || !chars[i - 1].is_ascii_alphanumeric() && chars[i - 1] != '.')
        {
            // Try to parse an IP octet sequence
            let mut octets = 0;
            let mut valid_ip = true;
            let mut curr = i;

            while octets < 4 && curr < len {
                let octet_start = curr;
                while curr < len && chars[curr].is_ascii_digit() {
                    curr += 1;
                }
                let octet_len = curr - octet_start;
                if octet_len == 0 || octet_len > 3 {
                    valid_ip = false;
                    break;
                }
                let num_str: String = chars[octet_start..curr].iter().collect();
                if let Ok(num) = num_str.parse::<u16>() {
                    if num > 255 {
                        valid_ip = false;
                        break;
                    }
                } else {
                    valid_ip = false;
                    break;
                }

                octets += 1;
                if octets < 4 {
                    if curr < len && chars[curr] == '.' {
                        curr += 1;
                    } else {
                        valid_ip = false;
                        break;
                    }
                }
            }

            // Must end without alphanumeric or dot
            if valid_ip
                && octets == 4
                && (curr == len || (!chars[curr].is_ascii_alphanumeric() && chars[curr] != '.'))
            {
                out.push_str("[REDACTED_IP]");
                i = curr;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_usernames() {
        let win = r"Crash report located at C:\Users\Alice\AppData\Local\Game\crash.dmp";
        let redacted_win = redact_usernames_in_paths(win);
        assert_eq!(
            redacted_win,
            r"Crash report located at C:\Users\[USERNAME]\AppData\Local\Game\crash.dmp"
        );

        let unix = "/home/bob/steamapps/common/Game/Data/Managed";
        let redacted_unix = redact_usernames_in_paths(unix);
        assert_eq!(
            redacted_unix,
            "/home/[USERNAME]/steamapps/common/Game/Data/Managed"
        );

        let mac = "/Users/charlie/Library/Application Support/Game";
        let redacted_mac = redact_usernames_in_paths(mac);
        assert_eq!(
            redacted_mac,
            "/Users/[USERNAME]/Library/Application Support/Game"
        );
    }

    #[test]
    fn test_redact_tokens_and_secrets() {
        let raw = "Connecting with token=secret123456 and Bearer eyJhbGciOi... to endpoint";
        let redacted = redact_tokens_and_secrets(raw);
        assert_eq!(
            redacted,
            "Connecting with token=[REDACTED] and Bearer [REDACTED] to endpoint"
        );
    }

    #[test]
    fn test_redact_ipv4() {
        let raw = "Server connected at 192.168.1.100:8080 or 10.0.0.1";
        let redacted = redact_ipv4_addresses(raw);
        assert_eq!(
            redacted,
            "Server connected at [REDACTED_IP]:8080 or [REDACTED_IP]"
        );
    }
}
