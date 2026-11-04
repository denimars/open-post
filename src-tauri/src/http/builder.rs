//! Converts an [`HttpRequest`] into the neutral [`PreparedRequest`] shape that
//! [`crate::http::client`] hands to reqwest.
//!
//! Keeping this step free of reqwest types makes it directly unit-testable.

use crate::error::{AppError, AppResult};
use crate::models::{Auth, HttpMethod, HttpRequest, KeyValue, RequestBody};
use base64::Engine;
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub enum PreparedBody {
    None,
    /// Raw bytes plus the `Content-Type` implied by the chosen format.
    Raw {
        content: String,
        mime: &'static str,
    },
    UrlEncoded(Vec<(String, String)>),
    Multipart(Vec<(String, String)>),
}

#[derive(Debug)]
pub struct PreparedRequest {
    pub method: HttpMethod,
    pub url: Url,
    /// User headers plus the ones derived from auth, in insertion order.
    pub headers: Vec<(String, String)>,
    pub body: PreparedBody,
    pub timeout_ms: Option<u64>,
}

fn enabled(pairs: &[KeyValue]) -> impl Iterator<Item = (&str, &str)> {
    pairs
        .iter()
        .filter(|kv| kv.enabled && !kv.key.trim().is_empty())
        .map(|kv| (kv.key.trim(), kv.value.as_str()))
}

/// Parses `raw` and appends the enabled `params` to its query string.
///
/// A URL without a scheme is assumed to be `http://`, matching what users
/// type in the address bar.
pub fn build_url(raw: &str, params: &[KeyValue]) -> AppResult<Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidUrl("the URL is empty".into()));
    }

    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };

    let mut url =
        Url::parse(&with_scheme).map_err(|e| AppError::InvalidUrl(format!("{trimmed} ({e})")))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::InvalidUrl(format!(
            "unsupported scheme `{}`, expected http or https",
            url.scheme()
        )));
    }
    if url.host_str().is_none() {
        return Err(AppError::InvalidUrl(format!("{trimmed} has no host")));
    }

    {
        let mut query = url.query_pairs_mut();
        for (key, value) in enabled(params) {
            query.append_pair(key, value);
        }
    }
    // `query_pairs_mut` leaves a trailing `?` when nothing was appended.
    if url.query() == Some("") {
        url.set_query(None);
    }
    Ok(url)
}

/// Header lines contributed by the auth setting (empty for `Auth::None`).
pub fn auth_headers(auth: &Auth) -> Vec<(String, String)> {
    match auth {
        Auth::None => Vec::new(),
        Auth::Bearer { token } if token.trim().is_empty() => Vec::new(),
        Auth::Bearer { token } => {
            vec![("Authorization".into(), format!("Bearer {}", token.trim()))]
        }
        Auth::Basic { username, password } => {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            vec![("Authorization".into(), format!("Basic {encoded}"))]
        }
    }
}

pub fn build_body(body: &RequestBody) -> PreparedBody {
    match body {
        RequestBody::None => PreparedBody::None,
        RequestBody::Raw {
            content,
            content_type,
        } => {
            if content.is_empty() {
                PreparedBody::None
            } else {
                PreparedBody::Raw {
                    content: content.clone(),
                    mime: content_type.mime(),
                }
            }
        }
        RequestBody::UrlEncoded { fields } => PreparedBody::UrlEncoded(
            enabled(fields)
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ),
        RequestBody::FormData { fields } => PreparedBody::Multipart(
            enabled(fields)
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ),
    }
}

/// Builds the full prepared request. Variables must already be resolved.
pub fn prepare(request: &HttpRequest) -> AppResult<PreparedRequest> {
    let url = build_url(&request.url, &request.params)?;

    let mut headers: Vec<(String, String)> = enabled(&request.headers)
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // A header the user typed wins over the generated auth header.
    let has_auth_header = headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("authorization"));
    if !has_auth_header {
        headers.extend(auth_headers(&request.auth));
    }

    Ok(PreparedRequest {
        method: request.method,
        url,
        headers,
        body: build_body(&request.body),
        timeout_ms: request.timeout_ms,
    })
}

/// Default `Content-Type` for a prepared body, if the body implies one.
pub fn body_content_type(body: &PreparedBody) -> Option<&'static str> {
    match body {
        PreparedBody::None | PreparedBody::Multipart(_) => None,
        PreparedBody::Raw { mime, .. } => Some(mime),
        PreparedBody::UrlEncoded(_) => Some("application/x-www-form-urlencoded"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RawContentType;

    fn kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }

    fn off(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: false,
        }
    }

    fn request(url: &str) -> HttpRequest {
        HttpRequest {
            id: "1".into(),
            method: HttpMethod::Get,
            url: url.into(),
            params: vec![],
            headers: vec![],
            body: RequestBody::None,
            auth: Auth::None,
            timeout_ms: None,
            name: None,
        }
    }

    #[test]
    fn appends_enabled_params_only() {
        let url = build_url(
            "https://api.test/users",
            &[kv("page", "2"), off("debug", "1")],
        )
        .expect("valid url");
        assert_eq!(url.as_str(), "https://api.test/users?page=2");
    }

    #[test]
    fn keeps_params_already_in_the_url() {
        let url = build_url("https://api.test/users?a=1", &[kv("b", "2")]).expect("valid url");
        assert_eq!(url.as_str(), "https://api.test/users?a=1&b=2");
    }

    #[test]
    fn percent_encodes_param_values() {
        let url = build_url("https://api.test/s", &[kv("q", "a b&c")]).expect("valid url");
        assert_eq!(url.as_str(), "https://api.test/s?q=a+b%26c");
    }

    #[test]
    fn no_params_leaves_no_trailing_question_mark() {
        let url = build_url("https://api.test/users", &[]).expect("valid url");
        assert_eq!(url.as_str(), "https://api.test/users");
    }

    #[test]
    fn assumes_http_when_scheme_is_missing() {
        let url = build_url("localhost:8080/health", &[]).expect("valid url");
        assert_eq!(url.as_str(), "http://localhost:8080/health");
    }

    #[test]
    fn rejects_empty_url() {
        let err = build_url("   ", &[]).expect_err("should reject");
        assert_eq!(err.kind(), "invalidUrl");
    }

    #[test]
    fn rejects_non_http_scheme() {
        let err = build_url("ftp://files.test/x", &[]).expect_err("should reject");
        assert_eq!(err.kind(), "invalidUrl");
    }

    #[test]
    fn bearer_auth_becomes_authorization_header() {
        let headers = auth_headers(&Auth::Bearer {
            token: "abc123".into(),
        });
        assert_eq!(
            headers,
            vec![("Authorization".into(), "Bearer abc123".into())]
        );
    }

    #[test]
    fn empty_bearer_token_adds_no_header() {
        assert!(auth_headers(&Auth::Bearer { token: "  ".into() }).is_empty());
    }

    #[test]
    fn basic_auth_is_base64_encoded() {
        let headers = auth_headers(&Auth::Basic {
            username: "user".into(),
            password: "pass".into(),
        });
        // "user:pass"
        assert_eq!(
            headers,
            vec![("Authorization".into(), "Basic dXNlcjpwYXNz".into())]
        );
    }

    #[test]
    fn explicit_authorization_header_wins_over_auth_setting() {
        let mut req = request("https://api.test");
        req.headers = vec![kv("authorization", "Custom xyz")];
        req.auth = Auth::Bearer {
            token: "abc".into(),
        };
        let prepared = prepare(&req).expect("prepared");
        let auth: Vec<_> = prepared
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .collect();
        assert_eq!(auth.len(), 1);
        assert_eq!(auth[0].1, "Custom xyz");
    }

    #[test]
    fn raw_json_body_carries_its_mime() {
        let body = build_body(&RequestBody::Raw {
            content: "{\"a\":1}".into(),
            content_type: RawContentType::Json,
        });
        assert_eq!(
            body,
            PreparedBody::Raw {
                content: "{\"a\":1}".into(),
                mime: "application/json",
            }
        );
        assert_eq!(body_content_type(&body), Some("application/json"));
    }

    #[test]
    fn empty_raw_body_is_none() {
        let body = build_body(&RequestBody::Raw {
            content: String::new(),
            content_type: RawContentType::Json,
        });
        assert_eq!(body, PreparedBody::None);
    }

    #[test]
    fn url_encoded_body_drops_disabled_fields() {
        let body = build_body(&RequestBody::UrlEncoded {
            fields: vec![kv("a", "1"), off("b", "2")],
        });
        assert_eq!(
            body,
            PreparedBody::UrlEncoded(vec![("a".into(), "1".into())])
        );
        assert_eq!(
            body_content_type(&body),
            Some("application/x-www-form-urlencoded")
        );
    }

    #[test]
    fn form_data_body_becomes_multipart() {
        let body = build_body(&RequestBody::FormData {
            fields: vec![kv("name", "ada")],
        });
        assert_eq!(
            body,
            PreparedBody::Multipart(vec![("name".into(), "ada".into())])
        );
        // reqwest sets the multipart boundary itself.
        assert_eq!(body_content_type(&body), None);
    }

    #[test]
    fn disabled_and_blank_headers_are_dropped() {
        let mut req = request("https://api.test");
        req.headers = vec![kv("X-A", "1"), off("X-B", "2"), kv("   ", "3")];
        let prepared = prepare(&req).expect("prepared");
        assert_eq!(prepared.headers, vec![("X-A".to_string(), "1".to_string())]);
    }
}
