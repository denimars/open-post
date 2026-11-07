//! Serde mirror of the Postman Collection Format v2.1.0 and of the Postman
//! environment export file.
//!
//! The structs are deliberately permissive: a file produced by Postman, Insomnia
//! or another tool may omit fields, use `null` where a string is expected, or
//! carry a shape from v2.0 (`url` as a plain string). Anything we cannot map is
//! ignored rather than rejected, so importing never fails over a field the MVP
//! does not use.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const COLLECTION_SCHEMA_V2_1: &str =
    "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";

/// Marks the files we write, so a round trip can tell them apart.
pub const EXPORTER_ID: &str = "open-post";

// ---------------------------------------------------------------- collection

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostmanCollection {
    pub info: Info,
    #[serde(default)]
    pub item: Vec<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<Variable>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Info {
    #[serde(
        rename = "_postman_id",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub postman_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: String,
    #[serde(
        rename = "_exporter_id",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exporter_id: Option<String>,
}

/// A collection entry: a folder when `item` is set, a request when `request` is.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Item {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Present on folders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<Vec<Item>>,
    /// Present on requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<Request>,
    /// Folder- or request-level auth, inherited by children in Postman.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    /// Fields Postman has no place for. Other tools ignore it; we read it back.
    #[serde(rename = "_openPost", default, skip_serializing_if = "Option::is_none")]
    pub open_post: Option<OpenPostExtras>,
}

/// open-post settings that the Postman format cannot express.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenPostExtras {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Request {
    #[serde(default)]
    pub method: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Header>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<UrlField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Header {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// `url` is an object in v2.1 and a bare string in v2.0.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum UrlField {
    Raw(String),
    Parsed(UrlObject),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UrlObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// String or array of segments, depending on the producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<Param>,
}

/// A query-string or url-encoded field.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Param {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FormParam {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
    /// `"text"` or `"file"`; file parts are not supported by the MVP.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Body {
    /// `none` | `raw` | `urlencoded` | `formdata` | `file` | `graphql`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urlencoded: Vec<Param>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formdata: Vec<FormParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<BodyOptions>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BodyOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawOptions>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RawOptions {
    /// `json` | `text` | `xml` | `html` | `javascript`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostmanAuth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer: Option<AuthParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic: Option<AuthParams>,
}

/// v2.1 writes a list of `{key, value}`; some exporters write a plain object.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AuthParams {
    List(Vec<AuthParam>),
    Map(serde_json::Map<String, Value>),
}

impl AuthParams {
    /// Looks a parameter up in either shape and renders it as a string.
    pub fn get(&self, key: &str) -> Option<String> {
        match self {
            AuthParams::List(items) => items
                .iter()
                .find(|item| item.key == key)
                .and_then(|item| item.value.as_ref())
                .map(value_to_string),
            AuthParams::Map(map) => map.get(key).map(value_to_string),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthParam {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Variable {
    #[serde(default)]
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

// --------------------------------------------------------------- environment

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostmanEnvironment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub values: Vec<EnvValue>,
    #[serde(
        rename = "_postman_variable_scope",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub scope: Option<String>,
    #[serde(
        rename = "_postman_exported_at",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exported_at: Option<String>,
    #[serde(
        rename = "_postman_exported_using",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exported_using: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EnvValue {
    #[serde(default)]
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Postman also writes `"secret"` here; we keep the value either way.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Renders a JSON scalar the way Postman displays it (no surrounding quotes).
pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Normalizes a `host`/`path` field that may be a string or an array.
pub fn join_segments(value: &Value, separator: &str) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => s.clone(),
                Value::Object(map) => map.get("value").map(value_to_string).unwrap_or_default(),
                other => value_to_string(other),
            })
            .collect::<Vec<_>>()
            .join(separator),
        other => value_to_string(other),
    }
}
