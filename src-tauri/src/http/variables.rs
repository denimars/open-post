//! `{{name}}` substitution against the active environment.
//!
//! Unknown variables are left verbatim in the output and collected so the UI
//! can warn about them.

use crate::models::{Auth, HttpRequest, KeyValue, RequestBody};
use std::collections::{BTreeSet, HashMap};

/// Maximum number of nested expansions, guards against `a -> {{b}} -> {{a}}`.
const MAX_PASSES: usize = 5;

pub struct Resolver {
    vars: HashMap<String, String>,
    unresolved: BTreeSet<String>,
}

impl Resolver {
    pub fn new(variables: &[KeyValue]) -> Self {
        let vars = variables
            .iter()
            .filter(|v| v.enabled && !v.key.trim().is_empty())
            .map(|v| (v.key.trim().to_string(), v.value.clone()))
            .collect();
        Resolver {
            vars,
            unresolved: BTreeSet::new(),
        }
    }

    /// Names referenced by the request that the environment did not define.
    pub fn unresolved(&self) -> Vec<String> {
        self.unresolved.iter().cloned().collect()
    }

    /// Replaces every `{{name}}` occurrence, repeating while substitutions
    /// introduce new placeholders (bounded by [`MAX_PASSES`]).
    pub fn resolve(&mut self, input: &str) -> String {
        let mut current = input.to_string();
        for _ in 0..MAX_PASSES {
            let (next, changed) = self.resolve_once(&current);
            current = next;
            if !changed {
                break;
            }
        }
        current
    }

    fn resolve_once(&mut self, input: &str) -> (String, bool) {
        let mut out = String::with_capacity(input.len());
        let mut rest = input;
        let mut changed = false;

        while let Some(start) = rest.find("{{") {
            let after_open = &rest[start + 2..];
            let Some(end) = after_open.find("}}") else {
                // No closing braces left; the remainder is literal text.
                break;
            };
            let raw_name = &after_open[..end];
            let name = raw_name.trim();

            out.push_str(&rest[..start]);
            match self.vars.get(name) {
                Some(value) if !name.is_empty() => {
                    out.push_str(value);
                    changed = true;
                }
                _ => {
                    if !name.is_empty() {
                        self.unresolved.insert(name.to_string());
                    }
                    // Unknown: keep the placeholder exactly as written.
                    out.push_str("{{");
                    out.push_str(raw_name);
                    out.push_str("}}");
                }
            }
            rest = &after_open[end + 2..];
        }
        out.push_str(rest);
        (out, changed)
    }

    fn resolve_pairs(&mut self, pairs: &[KeyValue]) -> Vec<KeyValue> {
        pairs
            .iter()
            .map(|kv| KeyValue {
                key: self.resolve(&kv.key),
                value: self.resolve(&kv.value),
                enabled: kv.enabled,
            })
            .collect()
    }

    /// Returns a copy of `request` with every user-supplied string resolved.
    pub fn resolve_request(&mut self, request: &HttpRequest) -> HttpRequest {
        let body = match &request.body {
            RequestBody::None => RequestBody::None,
            RequestBody::Raw {
                content,
                content_type,
            } => RequestBody::Raw {
                content: self.resolve(content),
                content_type: *content_type,
            },
            RequestBody::UrlEncoded { fields } => RequestBody::UrlEncoded {
                fields: self.resolve_pairs(fields),
            },
            RequestBody::FormData { fields } => RequestBody::FormData {
                fields: self.resolve_pairs(fields),
            },
        };

        let auth = match &request.auth {
            Auth::None => Auth::None,
            Auth::Bearer { token } => Auth::Bearer {
                token: self.resolve(token),
            },
            Auth::Basic { username, password } => Auth::Basic {
                username: self.resolve(username),
                password: self.resolve(password),
            },
        };

        HttpRequest {
            id: request.id.clone(),
            method: request.method,
            url: self.resolve(&request.url),
            params: self.resolve_pairs(&request.params),
            headers: self.resolve_pairs(&request.headers),
            body,
            auth,
            timeout_ms: request.timeout_ms,
            name: request.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }

    #[test]
    fn substitutes_known_variables() {
        let mut r = Resolver::new(&[kv("base_url", "https://api.test")]);
        assert_eq!(r.resolve("{{base_url}}/users"), "https://api.test/users");
        assert!(r.unresolved().is_empty());
    }

    #[test]
    fn leaves_unknown_variables_and_reports_them() {
        let mut r = Resolver::new(&[]);
        assert_eq!(r.resolve("{{missing}}/x"), "{{missing}}/x");
        assert_eq!(r.unresolved(), vec!["missing".to_string()]);
    }

    #[test]
    fn mixes_known_and_unknown() {
        let mut r = Resolver::new(&[kv("a", "1")]);
        assert_eq!(r.resolve("{{a}}-{{b}}-{{a}}"), "1-{{b}}-1");
        assert_eq!(r.unresolved(), vec!["b".to_string()]);
    }

    #[test]
    fn trims_whitespace_inside_braces() {
        let mut r = Resolver::new(&[kv("token", "abc")]);
        assert_eq!(r.resolve("Bearer {{ token }}"), "Bearer abc");
    }

    #[test]
    fn ignores_disabled_variables() {
        let mut r = Resolver::new(&[KeyValue {
            key: "a".into(),
            value: "1".into(),
            enabled: false,
        }]);
        assert_eq!(r.resolve("{{a}}"), "{{a}}");
    }

    #[test]
    fn resolves_nested_variables() {
        let mut r = Resolver::new(&[kv("host", "api.test"), kv("base_url", "https://{{host}}")]);
        assert_eq!(r.resolve("{{base_url}}/v1"), "https://api.test/v1");
    }

    #[test]
    fn cyclic_variables_terminate() {
        let mut r = Resolver::new(&[kv("a", "{{b}}"), kv("b", "{{a}}")]);
        let out = r.resolve("{{a}}");
        assert!(out.contains("{{"));
    }

    #[test]
    fn unclosed_placeholder_is_left_alone() {
        let mut r = Resolver::new(&[kv("a", "1")]);
        assert_eq!(r.resolve("{{a"), "{{a");
    }

    #[test]
    fn handles_text_without_placeholders() {
        let mut r = Resolver::new(&[kv("a", "1")]);
        assert_eq!(r.resolve("plain text"), "plain text");
    }
}
