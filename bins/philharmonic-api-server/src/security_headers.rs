use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

const X_CONTENT_TYPE_OPTIONS: &str = "x-content-type-options";
const X_FRAME_OPTIONS: &str = "x-frame-options";
const REFERRER_POLICY: &str = "referrer-policy";
const PERMISSIONS_POLICY: &str = "permissions-policy";
const ACCESS_CONTROL_ALLOW_ORIGIN: &str = "access-control-allow-origin";
const ACCESS_CONTROL_ALLOW_METHODS: &str = "access-control-allow-methods";
const ACCESS_CONTROL_ALLOW_HEADERS: &str = "access-control-allow-headers";
const ACCESS_CONTROL_MAX_AGE: &str = "access-control-max-age";

// CORS posture: `*` allow-origin so browser clients on any
// origin can call the API directly. This is safe because all
// authentication is carried by explicit `Authorization: Bearer`
// headers, never by ambient credentials (cookies, HTTP auth
// state, client certs) — fetch() defaults to `credentials:
// 'same-origin'` everywhere we control, so `*` does not enable
// any credentialed cross-origin escalation. Preflight `OPTIONS`
// short-circuits to 204 in this middleware so the browser sees
// a 2xx (Axum's per-route handlers don't list OPTIONS, which
// would otherwise 405 the preflight and stop the real request).
const ALLOW_METHODS: &str = "GET, POST, PATCH, PUT, DELETE, OPTIONS";
const ALLOW_HEADERS: &str = "Authorization, Content-Type, X-Tenant-Id";
const MAX_AGE: &str = "86400";

pub(crate) async fn inject(request: axum::extract::Request, next: Next) -> Response {
    if request.method() == Method::OPTIONS {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NO_CONTENT;
        apply_headers(response.headers_mut());
        return response;
    }

    let mut response = next.run(request).await;
    apply_headers(response.headers_mut());
    response
}

fn apply_headers(headers: &mut HeaderMap) {
    headers
        .entry(X_CONTENT_TYPE_OPTIONS)
        .or_insert(HeaderValue::from_static("nosniff"));
    headers
        .entry(X_FRAME_OPTIONS)
        .or_insert(HeaderValue::from_static("DENY"));
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    headers
        .entry(REFERRER_POLICY)
        .or_insert(HeaderValue::from_static("strict-origin-when-cross-origin"));
    headers
        .entry(PERMISSIONS_POLICY)
        .or_insert(HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=()",
        ));
    headers
        .entry(ACCESS_CONTROL_ALLOW_ORIGIN)
        .or_insert(HeaderValue::from_static("*"));
    headers
        .entry(ACCESS_CONTROL_ALLOW_METHODS)
        .or_insert(HeaderValue::from_static(ALLOW_METHODS));
    headers
        .entry(ACCESS_CONTROL_ALLOW_HEADERS)
        .or_insert(HeaderValue::from_static(ALLOW_HEADERS));
    headers
        .entry(ACCESS_CONTROL_MAX_AGE)
        .or_insert(HeaderValue::from_static(MAX_AGE));
}
