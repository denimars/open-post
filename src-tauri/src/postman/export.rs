//! open-post -> Postman Collection v2.1.0 / Postman environment.

use super::schema::*;
use crate::error::AppResult;
use crate::models::{Auth, Collection, Environment, HttpRequest, KeyValue, RawContentType};
use crate::models::{RequestBody, SavedRequest};

/// How many `/`-separated levels of a saved request name become folders.
/// Beyond that the remainder stays in the request name.
const MAX_FOLDER_DEPTH: usize = 5;

/// Serializes a collection as a Postman v2.1.0 collection file.
pub fn collection_json(collection: &Collection) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(&to_postman(collection))?)
}

/// Serializes an environment as a Postman environment file.
pub fn environment_json(environment: &Environment) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(&to_postman_environment(
        environment,
    ))?)
}

pub fn to_postman(collection: &Collection) -> PostmanCollection {
    let entries = collection
        .requests
        .iter()
        .map(|saved| (folder_path(&saved.name), request_item(saved)))
        .collect();

    PostmanCollection {
        info: Info {
            postman_id: Some(collection.id.clone()),
            name: collection.name.clone(),
            description: None,
            schema: COLLECTION_SCHEMA_V2_1.to_string(),
            exporter_id: Some(EXPORTER_ID.to_string()),
        },
        item: group_items(entries),
        auth: None,
        variable: vec![],
    }
}

pub fn to_postman_environment(environment: &Environment) -> PostmanEnvironment {
    PostmanEnvironment {
        id: Some(environment.id.clone()),
        name: environment.name.clone(),
        values: environment
            .variables
            .iter()
            .filter(|variable| !variable.key.trim().is_empty())
            .map(|variable| EnvValue {
                key: variable.key.clone(),
                value: Some(serde_json::Value::String(variable.value.clone())),
                enabled: Some(variable.enabled),
                kind: Some("default".to_string()),
            })
            .collect(),
        scope: Some("environment".to_string()),
        exported_at: None,
        exported_using: Some(EXPORTER_ID.to_string()),
    }
}

/// Splits `Users/Create user` into the folder path and leaves the leaf name.
/// Empty segments are dropped so `a//b` behaves like `a/b`.
fn folder_path(name: &str) -> Vec<String> {
    let mut segments: Vec<String> = name
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();
    if segments.len() <= 1 {
        return vec![];
    }
    segments.pop();
    segments.truncate(MAX_FOLDER_DEPTH);
    segments
}

fn leaf_name(name: &str) -> String {
    let segments: Vec<&str> = name
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    match segments.len() {
        0 => name.trim().to_string(),
        // Everything past the folder depth stays part of the request name.
        _ if segments.len() > MAX_FOLDER_DEPTH + 1 => segments[MAX_FOLDER_DEPTH..].join("/"),
        _ => segments[segments.len() - 1].to_string(),
    }
}

/// An item plus the folder path it belongs under.
type Entry = (Vec<String>, Item);
/// A folder name plus the entries collected for it.
type Folder = (String, Vec<Entry>);

enum Slot {
    Leaf(Box<Item>),
    Folder(usize),
}

/// Rebuilds the folder tree from `(folder path, item)` pairs, keeping each
/// folder at the position where it first appears.
fn group_items(entries: Vec<Entry>) -> Vec<Item> {
    let mut slots: Vec<Slot> = Vec::new();
    let mut folders: Vec<Folder> = Vec::new();

    for (path, item) in entries {
        let Some((head, rest)) = path.split_first() else {
            slots.push(Slot::Leaf(Box::new(item)));
            continue;
        };
        let child = (rest.to_vec(), item);
        match folders.iter().position(|(name, _)| name == head) {
            Some(index) => folders[index].1.push(child),
            None => {
                folders.push((head.clone(), vec![child]));
                slots.push(Slot::Folder(folders.len() - 1));
            }
        }
    }

    let mut out = Vec::with_capacity(slots.len());
    for slot in slots {
        match slot {
            Slot::Leaf(item) => out.push(*item),
            Slot::Folder(index) => {
                let (name, children) = std::mem::take(&mut folders[index]);
                out.push(Item {
                    name: Some(name),
                    item: Some(group_items(children)),
                    ..Item::default()
                });
            }
        }
    }
    out
}

fn request_item(saved: &SavedRequest) -> Item {
    let request = &saved.request;
    Item {
        name: Some(leaf_name(&saved.name)),
        item: None,
        request: Some(Request {
            method: request.method.as_str().to_string(),
            header: headers(&request.headers),
            url: Some(UrlField::Parsed(url_object(request))),
            body: body(&request.body),
            auth: auth(&request.auth),
            description: None,
        }),
        auth: None,
        open_post: request.timeout_ms.map(|timeout_ms| OpenPostExtras {
            timeout_ms: Some(timeout_ms),
        }),
    }
}

fn headers(headers: &[KeyValue]) -> Vec<Header> {
    headers
        .iter()
        .filter(|header| !header.key.trim().is_empty())
        .map(|header| Header {
            key: header.key.clone(),
            value: Some(header.value.clone()),
            disabled: (!header.enabled).then_some(true),
            kind: Some("text".to_string()),
        })
        .collect()
}

/// `raw` already carries the enabled params in its query string (the URL bar is
/// the source of truth); `query` is the parsed view Postman shows in its table,
/// including the rows the user disabled. That mirrors Postman's own convention.
fn url_object(request: &HttpRequest) -> UrlObject {
    UrlObject {
        raw: Some(request.url.clone()),
        query: params(&request.params),
        ..UrlObject::default()
    }
}

fn params(params: &[KeyValue]) -> Vec<Param> {
    params
        .iter()
        .filter(|param| !param.key.trim().is_empty())
        .map(|param| Param {
            key: param.key.clone(),
            value: Some(param.value.clone()),
            disabled: (!param.enabled).then_some(true),
        })
        .collect()
}

fn body(body: &RequestBody) -> Option<Body> {
    match body {
        RequestBody::None => None,
        RequestBody::Raw {
            content,
            content_type,
        } => Some(Body {
            mode: Some("raw".to_string()),
            raw: Some(content.clone()),
            options: Some(BodyOptions {
                raw: Some(RawOptions {
                    language: Some(language(*content_type).to_string()),
                }),
            }),
            ..Body::default()
        }),
        RequestBody::UrlEncoded { fields } => Some(Body {
            mode: Some("urlencoded".to_string()),
            urlencoded: params(fields),
            ..Body::default()
        }),
        RequestBody::FormData { fields } => Some(Body {
            mode: Some("formdata".to_string()),
            formdata: fields
                .iter()
                .filter(|field| !field.key.trim().is_empty())
                .map(|field| FormParam {
                    key: field.key.clone(),
                    value: Some(field.value.clone()),
                    kind: Some("text".to_string()),
                    src: None,
                    disabled: (!field.enabled).then_some(true),
                })
                .collect(),
            ..Body::default()
        }),
    }
}

fn language(content_type: RawContentType) -> &'static str {
    match content_type {
        RawContentType::Json => "json",
        RawContentType::Text => "text",
        RawContentType::Xml => "xml",
    }
}

fn auth(auth: &Auth) -> Option<PostmanAuth> {
    match auth {
        Auth::None => None,
        Auth::Bearer { token } => Some(PostmanAuth {
            kind: "bearer".to_string(),
            bearer: Some(AuthParams::List(vec![AuthParam {
                key: "token".to_string(),
                value: Some(serde_json::Value::String(token.clone())),
                kind: Some("string".to_string()),
            }])),
            basic: None,
        }),
        Auth::Basic { username, password } => Some(PostmanAuth {
            kind: "basic".to_string(),
            bearer: None,
            basic: Some(AuthParams::List(vec![
                AuthParam {
                    key: "username".to_string(),
                    value: Some(serde_json::Value::String(username.clone())),
                    kind: Some("string".to_string()),
                },
                AuthParam {
                    key: "password".to_string(),
                    value: Some(serde_json::Value::String(password.clone())),
                    kind: Some("string".to_string()),
                },
            ])),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;
    use serde_json::Value;

    fn saved(name: &str, request: HttpRequest) -> SavedRequest {
        SavedRequest {
            id: format!("saved-{name}"),
            collection_id: "c1".into(),
            name: name.into(),
            request,
        }
    }

    fn request() -> HttpRequest {
        HttpRequest {
            id: "r1".into(),
            method: HttpMethod::Get,
            url: "https://{{base_url}}/users".into(),
            params: vec![],
            headers: vec![],
            body: RequestBody::None,
            auth: Auth::None,
            timeout_ms: None,
            name: None,
        }
    }

    fn collection(requests: Vec<SavedRequest>) -> Collection {
        Collection {
            id: "c1".into(),
            name: "My API".into(),
            requests,
        }
    }

    #[test]
    fn writes_the_v2_1_schema_and_name() {
        let postman = to_postman(&collection(vec![saved("Ping", request())]));
        assert_eq!(postman.info.schema, COLLECTION_SCHEMA_V2_1);
        assert_eq!(postman.info.name, "My API");
        assert_eq!(postman.info.exporter_id.as_deref(), Some("open-post"));
        assert_eq!(postman.item.len(), 1);
    }

    #[test]
    fn a_slash_in_the_name_becomes_a_folder() {
        let postman = to_postman(&collection(vec![
            saved("Users/List", request()),
            saved("Users/Create", request()),
            saved("Ping", request()),
        ]));

        assert_eq!(postman.item.len(), 2);
        let folder = &postman.item[0];
        assert_eq!(folder.name.as_deref(), Some("Users"));
        let children = folder.item.as_ref().expect("folder children");
        let names: Vec<_> = children.iter().filter_map(|c| c.name.as_deref()).collect();
        assert_eq!(names, vec!["List", "Create"]);
        assert_eq!(postman.item[1].name.as_deref(), Some("Ping"));
        assert!(postman.item[1].request.is_some());
    }

    #[test]
    fn nested_folders_keep_their_order() {
        let postman = to_postman(&collection(vec![
            saved("v1/Users/List", request()),
            saved("v1/Health", request()),
        ]));
        let v1 = &postman.item[0];
        assert_eq!(v1.name.as_deref(), Some("v1"));
        let children = v1.item.as_ref().expect("children");
        assert_eq!(children[0].name.as_deref(), Some("Users"));
        assert_eq!(children[1].name.as_deref(), Some("Health"));
    }

    #[test]
    fn the_raw_url_keeps_variables_and_the_query_table_keeps_disabled_rows() {
        let mut req = request();
        req.url = "https://{{base_url}}/users?page=1".into();
        req.params = vec![
            KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
            },
            KeyValue {
                key: "debug".into(),
                value: "true".into(),
                enabled: false,
            },
        ];
        let postman = to_postman(&collection(vec![saved("List", req)]));
        let url = match postman.item[0]
            .request
            .as_ref()
            .expect("request")
            .url
            .as_ref()
        {
            Some(UrlField::Parsed(object)) => object.clone(),
            other => panic!("expected a parsed url, got {other:?}"),
        };

        assert_eq!(
            url.raw.as_deref(),
            Some("https://{{base_url}}/users?page=1")
        );
        assert_eq!(url.query.len(), 2);
        assert_eq!(url.query[0].disabled, None);
        assert_eq!(url.query[1].disabled, Some(true));
    }

    #[test]
    fn a_raw_body_declares_its_language() {
        let mut req = request();
        req.body = RequestBody::Raw {
            content: "{\"a\":1}".into(),
            content_type: RawContentType::Json,
        };
        let postman = to_postman(&collection(vec![saved("Create", req)]));
        let body = postman.item[0]
            .request
            .as_ref()
            .and_then(|r| r.body.as_ref())
            .expect("body");
        assert_eq!(body.mode.as_deref(), Some("raw"));
        assert_eq!(
            body.options
                .as_ref()
                .and_then(|o| o.raw.as_ref())
                .and_then(|r| r.language.as_deref()),
            Some("json")
        );
    }

    #[test]
    fn bearer_auth_uses_the_postman_parameter_list() {
        let mut req = request();
        req.auth = Auth::Bearer {
            token: "{{token}}".into(),
        };
        let postman = to_postman(&collection(vec![saved("Me", req)]));
        let auth = postman.item[0]
            .request
            .as_ref()
            .and_then(|r| r.auth.as_ref())
            .expect("auth");
        assert_eq!(auth.kind, "bearer");
        assert_eq!(
            auth.bearer.as_ref().and_then(|p| p.get("token")),
            Some("{{token}}".to_string())
        );
    }

    #[test]
    fn the_timeout_travels_in_the_open_post_extension() {
        let mut req = request();
        req.timeout_ms = Some(5_000);
        let postman = to_postman(&collection(vec![saved("Slow", req)]));
        assert_eq!(
            postman.item[0]
                .open_post
                .as_ref()
                .and_then(|extras| extras.timeout_ms),
            Some(5_000)
        );
    }

    #[test]
    fn the_exported_json_is_shaped_like_a_postman_file() {
        let json = collection_json(&collection(vec![saved("Ping", request())])).expect("json");
        let value: Value = serde_json::from_str(&json).expect("valid json");
        assert!(value["info"]["schema"]
            .as_str()
            .expect("schema")
            .contains("v2.1.0"));
        assert_eq!(value["item"][0]["request"]["method"], "GET");
        // `null`s would make the file noisy in a diff and in Postman's UI.
        assert!(!json.contains(": null"));
    }

    #[test]
    fn an_environment_is_exported_in_postman_shape() {
        let environment = Environment {
            id: "e1".into(),
            name: "Staging".into(),
            variables: vec![
                KeyValue {
                    key: "base_url".into(),
                    value: "https://api.staging.test".into(),
                    enabled: true,
                },
                KeyValue {
                    key: "unused".into(),
                    value: String::new(),
                    enabled: false,
                },
            ],
        };
        let value: Value =
            serde_json::from_str(&environment_json(&environment).expect("json")).expect("json");

        assert_eq!(value["name"], "Staging");
        assert_eq!(value["_postman_variable_scope"], "environment");
        assert_eq!(value["values"][0]["key"], "base_url");
        assert_eq!(value["values"][0]["enabled"], true);
        assert_eq!(value["values"][1]["enabled"], false);
    }
}
