use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, StatusCode, header};
use axum::response::IntoResponse;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../philharmonic/webui/dist/"]
struct Assets;

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("map") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn serve_asset(path: &str) -> Option<Response<Body>> {
    let file = Assets::get(path)?;
    let body = Body::from(file.data);
    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(mime_for(path)),
    );
    Some(response)
}

pub async fn webui_fallback(request: Request<Body>) -> Response<Body> {
    let path = request.uri().path().trim_start_matches('/');

    if let Some(response) = serve_asset(path) {
        return response;
    }

    // SPA fallback: serve index.html for any path that doesn't
    // match a static asset (React Router handles client-side routing).
    serve_asset("index.html")
        .unwrap_or_else(|| (StatusCode::NOT_FOUND, "webui not found").into_response())
}
