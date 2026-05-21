use axum::{
    body::Body,
    http::{HeaderValue, Request, Response, StatusCode, header},
    response::IntoResponse,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "dist/"]
struct Assets;

pub(crate) async fn serve(request: Request<Body>) -> Response<Body> {
    let path = request.uri().path().trim_start_matches('/');
    if let Some(response) = serve_asset(path) {
        return response;
    }

    serve_asset("index.html")
        .unwrap_or_else(|| (StatusCode::NOT_FOUND, "frontend bundle not found").into_response())
}

fn serve_asset(path: &str) -> Option<Response<Body>> {
    let asset_path = if path.is_empty() { "index.html" } else { path };
    let file = Assets::get(asset_path)?;
    let mut response = Response::new(Body::from(file.data));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(mime_for(asset_path)),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Some(response)
}

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
