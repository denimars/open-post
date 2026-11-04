//! Executes a prepared request with reqwest, with cancellation and timeout.

use crate::error::{AppError, AppResult};
use crate::http::builder::{self, PreparedBody, PreparedRequest};
use crate::http::variables::Resolver;
use crate::models::{HttpMethod, HttpRequest, HttpResponse, KeyValue};
use base64::Engine;
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// Bodies larger than this are truncated before being sent to the webview.
pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// The shared reqwest client, held in Tauri state.
pub fn build_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("open-post/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::Internal(format!("could not create the HTTP client: {e}")))
}

fn method_of(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    }
}

fn map_reqwest_error(e: &reqwest::Error, timeout_ms: u64) -> AppError {
    if e.is_timeout() {
        return AppError::Timeout(timeout_ms);
    }
    // reqwest wraps the transport error; inspect the chain for a cause.
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(e);
    let mut chain = String::new();
    while let Some(err) = source {
        chain.push_str(&err.to_string().to_lowercase());
        chain.push(' ');
        source = err.source();
    }

    if chain.contains("dns") || chain.contains("name or service not known") {
        AppError::Dns(e.to_string())
    } else if chain.contains("certificate")
        || chain.contains("tls")
        || chain.contains("ssl")
        || chain.contains("handshake")
    {
        AppError::Tls(e.to_string())
    } else if e.is_connect() || chain.contains("connection refused") {
        AppError::Connect(e.to_string())
    } else {
        AppError::Request(e.to_string())
    }
}

fn apply(
    client: &reqwest::Client,
    prepared: PreparedRequest,
    timeout_ms: u64,
) -> AppResult<reqwest::RequestBuilder> {
    let mut builder = client
        .request(method_of(prepared.method), prepared.url)
        .timeout(std::time::Duration::from_millis(timeout_ms));

    let user_set_content_type = prepared
        .headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));

    for (key, value) in &prepared.headers {
        builder = builder.header(key, value);
    }

    if !user_set_content_type {
        if let Some(mime) = builder::body_content_type(&prepared.body) {
            builder = builder.header("Content-Type", mime);
        }
    }

    builder = match prepared.body {
        PreparedBody::None => builder,
        PreparedBody::Raw { content, .. } => builder.body(content),
        PreparedBody::UrlEncoded(fields) => {
            // `.form()` would overwrite an explicit Content-Type header.
            builder.body(serde_urlencoded::to_string(&fields).map_err(|e| {
                AppError::InvalidRequest(format!("could not encode the form body: {e}"))
            })?)
        }
        PreparedBody::Multipart(fields) => {
            let mut form = reqwest::multipart::Form::new();
            for (key, value) in fields {
                form = form.text(key, value);
            }
            builder.multipart(form)
        }
    };

    Ok(builder)
}

/// Runs `request` (variables already resolved by the caller via [`Resolver`]).
async fn execute(
    client: &reqwest::Client,
    request: &HttpRequest,
    unresolved: Vec<String>,
    cancel: CancellationToken,
) -> AppResult<HttpResponse> {
    let prepared = builder::prepare(request)?;
    let timeout_ms = request.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let builder = apply(client, prepared, timeout_ms)?;

    let started = Instant::now();

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AppError::Cancelled),
        result = builder.send() => result.map_err(|e| map_reqwest_error(&e, timeout_ms))?,
    };

    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value
                    .to_str()
                    .unwrap_or("<non-utf8 header value>")
                    .to_string(),
            )
        })
        .collect();

    let bytes = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AppError::Cancelled),
        result = response.bytes() => result.map_err(|e| map_reqwest_error(&e, timeout_ms))?,
    };

    let duration_ms = started.elapsed().as_millis() as u64;
    let size_bytes = bytes.len() as u64;

    let is_truncated = bytes.len() > MAX_BODY_BYTES;
    let shown = if is_truncated {
        &bytes[..MAX_BODY_BYTES]
    } else {
        &bytes[..]
    };

    let (body, is_binary) = match std::str::from_utf8(shown) {
        Ok(text) => (text.to_string(), false),
        Err(_) => (
            base64::engine::general_purpose::STANDARD.encode(shown),
            true,
        ),
    };

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text,
        headers,
        body,
        is_binary,
        is_truncated,
        size_bytes,
        duration_ms,
        unresolved_variables: unresolved,
    })
}

/// Resolves `{{variables}}` against `variables`, then sends the request.
///
/// Returns the resolved request too, so the caller can record what was
/// actually sent in the history.
pub async fn send(
    client: &reqwest::Client,
    request: &HttpRequest,
    variables: &[KeyValue],
    cancel: CancellationToken,
) -> (AppResult<HttpResponse>, HttpRequest) {
    let mut resolver = Resolver::new(variables);
    let resolved = resolver.resolve_request(request);
    let unresolved = resolver.unresolved();
    let result = execute(client, &resolved, unresolved, cancel).await;
    (result, resolved)
}

/// Registry of in-flight requests, keyed by `request_id`.
#[derive(Default)]
pub struct CancelRegistry {
    inner: std::sync::Mutex<std::collections::HashMap<String, CancellationToken>>,
}

impl CancelRegistry {
    pub fn register(self: &Arc<Self>, request_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut map) = self.inner.lock() {
            // A re-send for the same id supersedes the previous attempt.
            if let Some(previous) = map.insert(request_id.to_string(), token.clone()) {
                previous.cancel();
            }
        }
        token
    }

    pub fn finish(&self, request_id: &str) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(request_id);
        }
    }

    /// Returns true when there was a matching in-flight request.
    pub fn cancel(&self, request_id: &str) -> bool {
        let Ok(mut map) = self.inner.lock() else {
            return false;
        };
        match map.remove(request_id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }
}
