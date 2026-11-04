//! End-to-end tests for request execution against a local mock server.

use open_post_lib::error::AppError;
use open_post_lib::http::client;
use open_post_lib::models::{Auth, HttpMethod, HttpRequest, KeyValue, RawContentType, RequestBody};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.into(),
        value: value.into(),
        enabled: true,
    }
}

fn request(method: HttpMethod, url: String) -> HttpRequest {
    HttpRequest {
        id: "req-1".into(),
        method,
        url,
        params: vec![],
        headers: vec![],
        body: RequestBody::None,
        auth: Auth::None,
        timeout_ms: Some(5_000),
        name: None,
    }
}

async fn send(
    req: &HttpRequest,
    vars: &[KeyValue],
) -> Result<open_post_lib::models::HttpResponse, AppError> {
    let http = client::build_client().expect("client");
    client::send(&http, req, vars, CancellationToken::new())
        .await
        .0
}

#[tokio::test]
async fn performs_a_get_and_returns_the_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&server)
        .await;

    let response = send(
        &request(HttpMethod::Get, format!("{}/users", server.uri())),
        &[],
    )
    .await
    .expect("response");

    assert_eq!(response.status, 200);
    assert_eq!(response.status_text, "OK");
    assert_eq!(response.body, r#"{"ok":true}"#);
    assert!(!response.is_binary);
    assert_eq!(response.size_bytes, 11);
}

#[tokio::test]
async fn sends_query_params_from_the_params_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("q", "rust"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Get, format!("{}/search", server.uri()));
    req.params = vec![kv("q", "rust")];
    assert_eq!(send(&req, &[]).await.expect("response").status, 200);
}

#[tokio::test]
async fn posts_a_raw_json_body_with_its_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users"))
        .and(header("content-type", "application/json"))
        .and(body_string(r#"{"name":"ada"}"#))
        .respond_with(ResponseTemplate::new(201).set_body_string("created"))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Post, format!("{}/users", server.uri()));
    req.body = RequestBody::Raw {
        content: r#"{"name":"ada"}"#.into(),
        content_type: RawContentType::Json,
    };

    let response = send(&req, &[]).await.expect("response");
    assert_eq!(response.status, 201);
    assert_eq!(response.status_text, "Created");
}

#[tokio::test]
async fn sends_a_url_encoded_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string("a=1&b=two+words"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Post, server.uri());
    req.body = RequestBody::UrlEncoded {
        fields: vec![kv("a", "1"), kv("b", "two words")],
    };
    assert_eq!(send(&req, &[]).await.expect("response").status, 200);
}

#[tokio::test]
async fn sends_multipart_form_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Post, server.uri());
    req.body = RequestBody::FormData {
        fields: vec![kv("name", "ada")],
    };
    assert_eq!(send(&req, &[]).await.expect("response").status, 200);

    let received = &server.received_requests().await.expect("requests")[0];
    let content_type = received
        .headers
        .get("content-type")
        .expect("content-type")
        .to_str()
        .expect("utf-8");
    assert!(content_type.starts_with("multipart/form-data; boundary="));
}

#[tokio::test]
async fn applies_bearer_auth() {
    let server = MockServer::start().await;
    Mock::given(header("authorization", "Bearer secret-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Get, server.uri());
    req.auth = Auth::Bearer {
        token: "secret-token".into(),
    };
    assert_eq!(send(&req, &[]).await.expect("response").status, 200);
}

#[tokio::test]
async fn a_404_is_a_normal_response_not_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
        .mount(&server)
        .await;

    let response = send(&request(HttpMethod::Get, server.uri()), &[])
        .await
        .expect("a 404 should not be an Err");
    assert_eq!(response.status, 404);
    assert_eq!(response.status_text, "Not Found");
    assert_eq!(response.body, "nope");
}

#[tokio::test]
async fn a_500_is_a_normal_response_not_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    assert_eq!(
        send(&request(HttpMethod::Get, server.uri()), &[])
            .await
            .expect("response")
            .status,
        500
    );
}

#[tokio::test]
async fn resolves_variables_before_sending() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/users"))
        .and(header("authorization", "Bearer abc"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Get, "{{base_url}}/v1/users".into());
    req.auth = Auth::Bearer {
        token: "{{token}}".into(),
    };
    let vars = vec![kv("base_url", &server.uri()), kv("token", "abc")];

    let response = send(&req, &vars).await.expect("response");
    assert_eq!(response.status, 200);
    assert!(response.unresolved_variables.is_empty());
}

#[tokio::test]
async fn reports_unresolved_variables() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Get, server.uri());
    req.headers = vec![kv("X-Token", "{{missing}}")];

    let response = send(&req, &[]).await.expect("response");
    assert_eq!(response.unresolved_variables, vec!["missing".to_string()]);
}

#[tokio::test]
async fn times_out_with_a_timeout_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(800)))
        .mount(&server)
        .await;

    let mut req = request(HttpMethod::Get, server.uri());
    req.timeout_ms = Some(100);

    let err = send(&req, &[]).await.expect_err("should time out");
    assert_eq!(err.kind(), "timeout");
}

#[tokio::test]
async fn a_cancelled_request_reports_cancelled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(2_000)))
        .mount(&server)
        .await;

    let http = client::build_client().expect("client");
    let token = CancellationToken::new();
    let child = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        child.cancel();
    });

    let (result, _) =
        client::send(&http, &request(HttpMethod::Get, server.uri()), &[], token).await;
    assert_eq!(result.expect_err("should be cancelled").kind(), "cancelled");
}

#[tokio::test]
async fn a_connection_refused_is_reported_as_connect() {
    // Port 1 is reserved and nothing listens on it.
    let err = send(&request(HttpMethod::Get, "http://127.0.0.1:1/".into()), &[])
        .await
        .expect_err("should fail");
    assert_eq!(err.kind(), "connect");
}

#[tokio::test]
async fn an_invalid_url_is_rejected_before_sending() {
    let err = send(&request(HttpMethod::Get, "   ".into()), &[])
        .await
        .expect_err("should fail");
    assert_eq!(err.kind(), "invalidUrl");
}

#[tokio::test]
async fn a_non_utf8_body_is_returned_as_base64() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0xff, 0xfe, 0x00]))
        .mount(&server)
        .await;

    let response = send(&request(HttpMethod::Get, server.uri()), &[])
        .await
        .expect("response");
    assert!(response.is_binary);
    assert_eq!(response.body, "//4A");
}

#[tokio::test]
async fn returns_response_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).insert_header("x-custom", "value"))
        .mount(&server)
        .await;

    let response = send(&request(HttpMethod::Get, server.uri()), &[])
        .await
        .expect("response");
    assert!(response
        .headers
        .iter()
        .any(|(k, v)| k == "x-custom" && v == "value"));
}

#[tokio::test]
async fn supports_delete_and_head() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    assert_eq!(
        send(&request(HttpMethod::Delete, server.uri()), &[])
            .await
            .expect("response")
            .status,
        204
    );
    let head = send(&request(HttpMethod::Head, server.uri()), &[])
        .await
        .expect("response");
    assert_eq!(head.status, 200);
    assert_eq!(head.body, "");
}
