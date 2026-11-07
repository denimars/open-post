//! Postman Collection v2.1.0 / Postman environment -> open-post.
//!
//! The import is lenient: anything the MVP cannot represent (GraphQL bodies,
//! file uploads, OAuth) is dropped and reported as a warning instead of
//! failing the whole file. open-post has no nested folders, so a folder tree is
//! flattened into `Folder/Sub/Request name`, which [`super::export`] turns back
//! into folders on the way out.

use super::schema::*;
use crate::error::{AppError, AppResult};
use crate::models::{Auth, HttpMethod, HttpRequest, KeyValue, RawContentType, RequestBody};

/// Hard stop for pathological files; folders nested deeper are ignored.
const MAX_DEPTH: usize = 32;

/// A collection file, mapped onto open-post's model and ready to be stored.
#[derive(Debug)]
pub struct ImportedCollection {
    pub name: String,
    /// `(saved request name, request)`, in file order.
    pub requests: Vec<(String, HttpRequest)>,
    /// Collection-level `variable` entries, which Postman resolves like an
    /// environment.
    pub variables: Vec<KeyValue>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ImportedEnvironment {
    pub name: String,
    pub variables: Vec<KeyValue>,
    pub warnings: Vec<String>,
}

pub fn parse_collection(json: &str) -> AppResult<ImportedCollection> {
    require_object(json, "collection")?;
    let collection: PostmanCollection = serde_json::from_str(json).map_err(|e| {
        if looks_like_v1(json) {
            AppError::InvalidRequest(
                "this is a Postman v1 collection; re-export it from Postman as \
                 Collection v2.1 and try again"
                    .into(),
            )
        } else {
            AppError::InvalidRequest(format!("not a Postman v2.1 collection file: {e}"))
        }
    })?;

    let mut warnings = Vec::new();
    if !collection.info.schema.contains("v2.1") && !collection.info.schema.is_empty() {
        warnings.push(format!(
            "the file declares the schema `{}`; it was read as v2.1",
            collection.info.schema
        ));
    }

    let name = match collection.info.name.trim() {
        "" => "Imported collection".to_string(),
        name => name.to_string(),
    };

    let mut requests = Vec::new();
    walk(
        &collection.item,
        &[],
        collection.auth.as_ref(),
        0,
        &mut requests,
        &mut warnings,
    );

    if requests.is_empty() {
        warnings.push("the file contained no request that could be imported".to_string());
    }

    Ok(ImportedCollection {
        name,
        requests,
        variables: variables(&collection.variable),
        warnings,
    })
}

pub fn parse_environment(json: &str) -> AppResult<ImportedEnvironment> {
    require_object(json, "environment")?;
    let environment: PostmanEnvironment = serde_json::from_str(json)
        .map_err(|e| AppError::InvalidRequest(format!("not a Postman environment file: {e}")))?;

    let mut warnings = Vec::new();
    if environment
        .values
        .iter()
        .any(|value| value.kind.as_deref() == Some("secret"))
    {
        warnings.push(
            "some variables were marked secret in Postman; their values are stored \
             as plain text here"
                .to_string(),
        );
    }

    let variables: Vec<KeyValue> = environment
        .values
        .iter()
        .filter(|value| !value.key.trim().is_empty())
        .map(|value| KeyValue {
            key: value.key.trim().to_string(),
            value: value
                .value
                .as_ref()
                .map(value_to_string)
                .unwrap_or_default(),
            enabled: value.enabled.unwrap_or(true),
        })
        .collect();

    if variables.is_empty() {
        warnings.push("the file contained no variable".to_string());
    }

    Ok(ImportedEnvironment {
        name: match environment.name.trim() {
            "" => "Imported environment".to_string(),
            name => name.to_string(),
        },
        variables,
        warnings,
    })
}

/// Serde happily builds a struct out of a JSON array, so the top level has to
/// be checked explicitly or `[]` would import as an empty collection.
fn require_object(json: &str, what: &str) -> AppResult<()> {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(serde_json::Value::Object(_)) => Ok(()),
        Ok(_) => Err(AppError::InvalidRequest(format!(
            "this file is not a Postman {what}: its top level is not an object"
        ))),
        Err(e) => Err(AppError::InvalidRequest(format!(
            "this is not a JSON file: {e}"
        ))),
    }
}

fn looks_like_v1(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|value| {
            let object = value.as_object()?;
            Some(object.contains_key("requests") && !object.contains_key("info"))
        })
        .unwrap_or(false)
}

fn variables(entries: &[Variable]) -> Vec<KeyValue> {
    entries
        .iter()
        .filter(|variable| !variable.key.trim().is_empty())
        .map(|variable| KeyValue {
            key: variable.key.trim().to_string(),
            value: variable
                .value
                .as_ref()
                .map(value_to_string)
                .unwrap_or_default(),
            enabled: !variable.disabled.unwrap_or(false),
        })
        .collect()
}

/// Depth-first walk over the item tree, flattening folders into the name.
fn walk(
    items: &[Item],
    prefix: &[String],
    inherited_auth: Option<&PostmanAuth>,
    depth: usize,
    out: &mut Vec<(String, HttpRequest)>,
    warnings: &mut Vec<String>,
) {
    if depth > MAX_DEPTH {
        warnings.push(format!(
            "folders nested deeper than {MAX_DEPTH} levels were skipped"
        ));
        return;
    }

    for item in items {
        let name = item.name.as_deref().unwrap_or("").trim();
        let auth = item.auth.as_ref().or(inherited_auth);

        if let Some(children) = &item.item {
            let mut nested = prefix.to_vec();
            if !name.is_empty() {
                nested.push(sanitize_segment(name));
            }
            walk(children, &nested, auth, depth + 1, out, warnings);
            continue;
        }

        let Some(request) = &item.request else {
            continue;
        };

        let label = if name.is_empty() {
            "Untitled request"
        } else {
            name
        };
        let full_name = prefix
            .iter()
            .cloned()
            .chain(std::iter::once(sanitize_segment(label)))
            .collect::<Vec<_>>()
            .join("/");

        match map_request(request, auth, item.open_post.as_ref(), &full_name, warnings) {
            Some(mapped) => out.push((full_name, mapped)),
            None => continue,
        }
    }
}

/// `/` is the folder separator in a flattened name, so it cannot appear inside
/// a segment.
fn sanitize_segment(name: &str) -> String {
    name.replace('/', "-")
}

fn map_request(
    request: &Request,
    inherited_auth: Option<&PostmanAuth>,
    extras: Option<&OpenPostExtras>,
    label: &str,
    warnings: &mut Vec<String>,
) -> Option<HttpRequest> {
    let Some(method) = method(&request.method) else {
        warnings.push(format!(
            "`{label}` was skipped: the method `{}` is not supported",
            request.method
        ));
        return None;
    };

    let (url, mut params) = url(request.url.as_ref());
    if url.trim().is_empty() {
        warnings.push(format!("`{label}` was skipped: it has no URL"));
        return None;
    }
    // Keep the Params table a view of the query string the URL already carries.
    if params.is_empty() {
        params = params_from_query(&url);
    }

    Some(HttpRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method,
        url,
        params,
        headers: headers(&request.header),
        body: body(request.body.as_ref(), label, warnings),
        auth: resolve_auth(request.auth.as_ref(), inherited_auth, label, warnings),
        timeout_ms: extras.and_then(|extras| extras.timeout_ms),
        name: Some(label.to_string()),
    })
}

fn method(raw: &str) -> Option<HttpMethod> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "GET" => Some(HttpMethod::Get),
        "POST" => Some(HttpMethod::Post),
        "PUT" => Some(HttpMethod::Put),
        "PATCH" => Some(HttpMethod::Patch),
        "DELETE" => Some(HttpMethod::Delete),
        "HEAD" => Some(HttpMethod::Head),
        "OPTIONS" => Some(HttpMethod::Options),
        _ => None,
    }
}

/// Returns the raw URL plus the query rows Postman listed for it.
fn url(field: Option<&UrlField>) -> (String, Vec<KeyValue>) {
    match field {
        None => (String::new(), vec![]),
        Some(UrlField::Raw(raw)) => (raw.trim().to_string(), vec![]),
        Some(UrlField::Parsed(object)) => {
            let raw = match object.raw.as_deref().map(str::trim) {
                Some(raw) if !raw.is_empty() => raw.to_string(),
                _ => rebuild_url(object),
            };
            let params = object
                .query
                .iter()
                .filter(|param| !param.key.trim().is_empty())
                .map(|param| KeyValue {
                    key: param.key.clone(),
                    value: param.value.clone().unwrap_or_default(),
                    enabled: !param.disabled.unwrap_or(false),
                })
                .collect();
            (raw, params)
        }
    }
}

/// Reassembles a URL from the parsed parts, for files that omit `raw`.
fn rebuild_url(object: &UrlObject) -> String {
    let mut url = String::new();
    if let Some(protocol) = &object.protocol {
        url.push_str(protocol);
        url.push_str("://");
    }
    if let Some(host) = &object.host {
        url.push_str(&join_segments(host, "."));
    }
    if let Some(port) = &object.port {
        url.push(':');
        url.push_str(port);
    }
    if let Some(path) = &object.path {
        let path = join_segments(path, "/");
        if !path.is_empty() {
            if !path.starts_with('/') {
                url.push('/');
            }
            url.push_str(&path);
        }
    }
    let query: Vec<String> = object
        .query
        .iter()
        .filter(|param| !param.key.trim().is_empty() && !param.disabled.unwrap_or(false))
        .map(|param| format!("{}={}", param.key, param.value.clone().unwrap_or_default()))
        .collect();
    if !query.is_empty() {
        url.push('?');
        url.push_str(&query.join("&"));
    }
    url
}

fn params_from_query(url: &str) -> Vec<KeyValue> {
    let Some((_, query)) = url.split_once('?') else {
        return vec![];
    };
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            KeyValue {
                key: key.to_string(),
                value: value.to_string(),
                enabled: true,
            }
        })
        .filter(|param| !param.key.trim().is_empty())
        .collect()
}

fn headers(headers: &[Header]) -> Vec<KeyValue> {
    headers
        .iter()
        .filter(|header| !header.key.trim().is_empty())
        .map(|header| KeyValue {
            key: header.key.clone(),
            value: header.value.clone().unwrap_or_default(),
            enabled: !header.disabled.unwrap_or(false),
        })
        .collect()
}

fn body(body: Option<&Body>, label: &str, warnings: &mut Vec<String>) -> RequestBody {
    let Some(body) = body else {
        return RequestBody::None;
    };

    match body.mode.as_deref().unwrap_or("none") {
        "raw" => RequestBody::Raw {
            content: body.raw.clone().unwrap_or_default(),
            content_type: language(
                body.options
                    .as_ref()
                    .and_then(|options| options.raw.as_ref())
                    .and_then(|raw| raw.language.as_deref()),
            ),
        },
        "urlencoded" => RequestBody::UrlEncoded {
            fields: body
                .urlencoded
                .iter()
                .filter(|field| !field.key.trim().is_empty())
                .map(|field| KeyValue {
                    key: field.key.clone(),
                    value: field.value.clone().unwrap_or_default(),
                    enabled: !field.disabled.unwrap_or(false),
                })
                .collect(),
        },
        "formdata" => {
            let files: Vec<&str> = body
                .formdata
                .iter()
                .filter(|field| field.kind.as_deref() == Some("file"))
                .map(|field| field.key.as_str())
                .collect();
            if !files.is_empty() {
                warnings.push(format!(
                    "`{label}`: the file fields {} were dropped, file upload is not \
                     supported yet",
                    files.join(", ")
                ));
            }
            RequestBody::FormData {
                fields: body
                    .formdata
                    .iter()
                    .filter(|field| field.kind.as_deref() != Some("file"))
                    .filter(|field| !field.key.trim().is_empty())
                    .map(|field| KeyValue {
                        key: field.key.clone(),
                        value: field.value.clone().unwrap_or_default(),
                        enabled: !field.disabled.unwrap_or(false),
                    })
                    .collect(),
            }
        }
        "none" => RequestBody::None,
        other => {
            warnings.push(format!(
                "`{label}`: the `{other}` body is not supported and was dropped"
            ));
            RequestBody::None
        }
    }
}

fn language(language: Option<&str>) -> RawContentType {
    match language.unwrap_or("text") {
        "json" => RawContentType::Json,
        "xml" => RawContentType::Xml,
        _ => RawContentType::Text,
    }
}

/// The request's own auth wins, unless it is Postman's explicit `inherit`, in
/// which case the folder's or collection's auth applies.
fn resolve_auth(
    own: Option<&PostmanAuth>,
    inherited: Option<&PostmanAuth>,
    label: &str,
    warnings: &mut Vec<String>,
) -> Auth {
    let effective = match own {
        Some(auth) if auth.kind != "inherit" => Some(auth),
        _ => inherited,
    };
    map_auth(effective, label, warnings)
}

fn map_auth(auth: Option<&PostmanAuth>, label: &str, warnings: &mut Vec<String>) -> Auth {
    let Some(auth) = auth else {
        return Auth::None;
    };

    match auth.kind.as_str() {
        "noauth" | "inherit" => Auth::None,
        "bearer" => Auth::Bearer {
            token: auth
                .bearer
                .as_ref()
                .and_then(|params| params.get("token"))
                .unwrap_or_default(),
        },
        "basic" => {
            let get = |key: &str| {
                auth.basic
                    .as_ref()
                    .and_then(|params| params.get(key))
                    .unwrap_or_default()
            };
            Auth::Basic {
                username: get("username"),
                password: get("password"),
            }
        }
        other => {
            warnings.push(format!(
                "`{label}`: `{other}` auth is not supported, the request was imported \
                 without authentication"
            ));
            Auth::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in the shape Postman actually exports: nested folders, a parsed
    /// `url` object, collection auth and collection variables.
    const POSTMAN_FILE: &str = r#"{
      "info": {
        "_postman_id": "9f0f",
        "name": "Billing API",
        "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
      },
      "auth": {
        "type": "bearer",
        "bearer": [{ "key": "token", "value": "{{token}}", "type": "string" }]
      },
      "variable": [
        { "key": "base_url", "value": "https://api.billing.test" }
      ],
      "item": [
        {
          "name": "Invoices",
          "item": [
            {
              "name": "List invoices",
              "request": {
                "method": "GET",
                "header": [
                  { "key": "Accept", "value": "application/json" },
                  { "key": "X-Debug", "value": "1", "disabled": true }
                ],
                "url": {
                  "raw": "{{base_url}}/invoices?page=1",
                  "host": ["{{base_url}}"],
                  "path": ["invoices"],
                  "query": [
                    { "key": "page", "value": "1" },
                    { "key": "status", "value": "open", "disabled": true }
                  ]
                }
              }
            },
            {
              "name": "Create invoice",
              "request": {
                "method": "POST",
                "auth": {
                  "type": "basic",
                  "basic": [
                    { "key": "username", "value": "admin" },
                    { "key": "password", "value": "s3cret" }
                  ]
                },
                "body": {
                  "mode": "raw",
                  "raw": "{\"amount\":100}",
                  "options": { "raw": { "language": "json" } }
                },
                "url": { "raw": "{{base_url}}/invoices" }
              }
            }
          ]
        },
        {
          "name": "Health",
          "request": { "method": "GET", "url": "https://billing.test/health" }
        }
      ]
    }"#;

    #[test]
    fn reads_a_postman_collection() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");

        assert_eq!(imported.name, "Billing API");
        let names: Vec<&str> = imported
            .requests
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "Invoices/List invoices",
                "Invoices/Create invoice",
                "Health"
            ]
        );
    }

    #[test]
    fn folders_become_a_slash_separated_name() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");
        let (_, request) = &imported.requests[0];
        assert_eq!(request.name.as_deref(), Some("Invoices/List invoices"));
    }

    #[test]
    fn headers_and_query_rows_keep_their_disabled_flag() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");
        let (_, request) = &imported.requests[0];

        assert_eq!(request.url, "{{base_url}}/invoices?page=1");
        assert_eq!(request.headers.len(), 2);
        assert!(request.headers[0].enabled);
        assert!(!request.headers[1].enabled);
        assert_eq!(request.params.len(), 2);
        assert!(request.params[0].enabled);
        assert!(!request.params[1].enabled);
    }

    #[test]
    fn collection_auth_is_inherited_and_request_auth_wins() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");

        assert_eq!(
            imported.requests[0].1.auth,
            Auth::Bearer {
                token: "{{token}}".into()
            }
        );
        assert_eq!(
            imported.requests[1].1.auth,
            Auth::Basic {
                username: "admin".into(),
                password: "s3cret".into()
            }
        );
    }

    #[test]
    fn a_raw_json_body_keeps_its_content_type() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");
        assert_eq!(
            imported.requests[1].1.body,
            RequestBody::Raw {
                content: "{\"amount\":100}".into(),
                content_type: RawContentType::Json,
            }
        );
    }

    #[test]
    fn collection_variables_are_reported() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");
        assert_eq!(imported.variables.len(), 1);
        assert_eq!(imported.variables[0].key, "base_url");
        assert_eq!(imported.variables[0].value, "https://api.billing.test");
    }

    /// Collection format v2.0 writes `url` as a plain string.
    #[test]
    fn reads_a_url_given_as_a_string() {
        let imported = parse_collection(POSTMAN_FILE).expect("import");
        let (_, request) = &imported.requests[2];
        assert_eq!(request.url, "https://billing.test/health");
        assert_eq!(request.method, HttpMethod::Get);
    }

    /// A `url` object without `raw` has to be reassembled from its parts.
    #[test]
    fn rebuilds_a_url_that_has_no_raw_field() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{
            "name": "List",
            "request": {
              "method": "GET",
              "url": {
                "protocol": "https",
                "host": ["api", "test"],
                "port": "8443",
                "path": ["v1", "users"],
                "query": [{ "key": "page", "value": "2" }]
              }
            }
          }]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert_eq!(
            imported.requests[0].1.url,
            "https://api.test:8443/v1/users?page=2"
        );
    }

    /// With no `query` table, the params come from the raw query string so the
    /// Params tab is not empty.
    #[test]
    fn derives_params_from_the_raw_query_string() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{
            "name": "List",
            "request": { "method": "GET", "url": "https://api.test/u?page=2&q=a" }
          }]
        }"#;
        let imported = parse_collection(json).expect("import");
        let params = &imported.requests[0].1.params;
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].key, "page");
        assert_eq!(params[1].value, "a");
    }

    #[test]
    fn form_data_file_fields_are_dropped_with_a_warning() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{
            "name": "Upload",
            "request": {
              "method": "POST",
              "url": "https://api.test/upload",
              "body": {
                "mode": "formdata",
                "formdata": [
                  { "key": "title", "value": "Report", "type": "text" },
                  { "key": "file", "type": "file", "src": "/tmp/a.pdf" }
                ]
              }
            }
          }]
        }"#;
        let imported = parse_collection(json).expect("import");

        match &imported.requests[0].1.body {
            RequestBody::FormData { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].key, "title");
            }
            other => panic!("expected form data, got {other:?}"),
        }
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("file")));
    }

    #[test]
    fn an_unsupported_body_is_dropped_with_a_warning() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{
            "name": "Query",
            "request": {
              "method": "POST",
              "url": "https://api.test/graphql",
              "body": { "mode": "graphql", "graphql": { "query": "{ me }" } }
            }
          }]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert_eq!(imported.requests[0].1.body, RequestBody::None);
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("graphql")));
    }

    #[test]
    fn an_unsupported_auth_scheme_imports_without_authentication() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{
            "name": "Keyed",
            "request": {
              "method": "GET",
              "url": "https://api.test/u",
              "auth": { "type": "oauth2" }
            }
          }]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert_eq!(imported.requests[0].1.auth, Auth::None);
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("oauth2")));
    }

    #[test]
    fn a_request_with_an_unsupported_method_is_skipped() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [
            { "name": "Linked", "request": { "method": "LINK", "url": "https://api.test/u" } },
            { "name": "Ok", "request": { "method": "GET", "url": "https://api.test/u" } }
          ]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert_eq!(imported.requests.len(), 1);
        assert_eq!(imported.requests[0].0, "Ok");
        assert!(imported.warnings.iter().any(|w| w.contains("LINK")));
    }

    #[test]
    fn a_request_without_a_url_is_skipped() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{ "name": "Broken", "request": { "method": "GET" } }]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert!(imported.requests.is_empty());
        assert!(imported.warnings.iter().any(|w| w.contains("no URL")));
    }

    #[test]
    fn a_slash_inside_a_request_name_does_not_create_a_folder() {
        let json = r#"{
          "info": { "name": "x", "schema": "v2.1.0" },
          "item": [{ "name": "GET /users", "request": { "method": "GET", "url": "https://a.test" } }]
        }"#;
        let imported = parse_collection(json).expect("import");
        assert_eq!(imported.requests[0].0, "GET -users");
    }

    #[test]
    fn a_v1_collection_is_rejected_with_an_explanation() {
        let json = r#"{ "id": "abc", "name": "Old", "requests": [] }"#;
        let error = parse_collection(json).expect_err("should fail");
        assert_eq!(error.kind(), "invalidRequest");
        assert!(error.to_string().contains("v2.1"));
    }

    #[test]
    fn a_file_that_is_not_a_collection_is_rejected() {
        let error = parse_collection("not json at all").expect_err("should fail");
        assert_eq!(error.kind(), "invalidRequest");
    }

    #[test]
    fn an_empty_collection_imports_with_a_warning() {
        let json = r#"{ "info": { "name": "Empty", "schema": "v2.1.0" }, "item": [] }"#;
        let imported = parse_collection(json).expect("import");
        assert!(imported.requests.is_empty());
        assert!(imported.warnings.iter().any(|w| w.contains("no request")));
    }

    #[test]
    fn reads_a_postman_environment() {
        let json = r#"{
          "id": "env-1",
          "name": "Staging",
          "values": [
            { "key": "base_url", "value": "https://api.staging.test", "enabled": true },
            { "key": "token", "value": "abc", "type": "secret" },
            { "key": "old", "value": "x", "enabled": false }
          ],
          "_postman_variable_scope": "environment"
        }"#;
        let imported = parse_environment(json).expect("import");

        assert_eq!(imported.name, "Staging");
        assert_eq!(imported.variables.len(), 3);
        assert!(imported.variables[0].enabled);
        // Postman omits `enabled` for variables that are on.
        assert!(imported.variables[1].enabled);
        assert!(!imported.variables[2].enabled);
        assert!(imported.warnings.iter().any(|w| w.contains("secret")));
    }

    /// A number or boolean in a variable value is rendered, not quoted.
    #[test]
    fn non_string_variable_values_are_stringified() {
        let json = r#"{ "name": "x", "values": [{ "key": "port", "value": 8080 }] }"#;
        let imported = parse_environment(json).expect("import");
        assert_eq!(imported.variables[0].value, "8080");
    }

    #[test]
    fn an_environment_file_that_is_not_one_is_rejected() {
        let error = parse_environment("[]").expect_err("should fail");
        assert_eq!(error.kind(), "invalidRequest");
    }

    /// Export then import has to give back the same requests, including the
    /// folder structure carried in the names.
    #[test]
    fn a_collection_survives_a_round_trip() {
        use crate::models::{Collection, SavedRequest};

        let original = Collection {
            id: "c1".into(),
            name: "Round trip".into(),
            requests: vec![
                SavedRequest {
                    id: "s1".into(),
                    collection_id: "c1".into(),
                    name: "Users/Create".into(),
                    request: HttpRequest {
                        id: "r1".into(),
                        method: HttpMethod::Post,
                        url: "{{base_url}}/users?dry=1".into(),
                        params: vec![
                            KeyValue {
                                key: "dry".into(),
                                value: "1".into(),
                                enabled: true,
                            },
                            KeyValue {
                                key: "trace".into(),
                                value: "on".into(),
                                enabled: false,
                            },
                        ],
                        headers: vec![KeyValue {
                            key: "Accept".into(),
                            value: "application/json".into(),
                            enabled: true,
                        }],
                        body: RequestBody::Raw {
                            content: "{\"name\":\"a\"}".into(),
                            content_type: RawContentType::Json,
                        },
                        auth: Auth::Bearer {
                            token: "{{token}}".into(),
                        },
                        timeout_ms: Some(15_000),
                        name: Some("Users/Create".into()),
                    },
                },
                SavedRequest {
                    id: "s2".into(),
                    collection_id: "c1".into(),
                    name: "Ping".into(),
                    request: HttpRequest {
                        id: "r2".into(),
                        method: HttpMethod::Get,
                        url: "https://a.test/ping".into(),
                        params: vec![],
                        headers: vec![],
                        body: RequestBody::UrlEncoded {
                            fields: vec![KeyValue {
                                key: "a".into(),
                                value: "b".into(),
                                enabled: true,
                            }],
                        },
                        auth: Auth::None,
                        timeout_ms: None,
                        name: None,
                    },
                },
            ],
        };

        let json = crate::postman::export::collection_json(&original).expect("export");
        let imported = parse_collection(&json).expect("import");

        assert_eq!(imported.name, "Round trip");
        assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
        assert_eq!(imported.requests.len(), 2);

        for (index, (name, request)) in imported.requests.iter().enumerate() {
            let source = &original.requests[index];
            assert_eq!(name, &source.name);
            assert_eq!(request.method, source.request.method);
            assert_eq!(request.url, source.request.url);
            assert_eq!(request.params, source.request.params);
            assert_eq!(request.headers, source.request.headers);
            assert_eq!(request.body, source.request.body);
            assert_eq!(request.auth, source.request.auth);
            assert_eq!(request.timeout_ms, source.request.timeout_ms);
        }
    }
}
