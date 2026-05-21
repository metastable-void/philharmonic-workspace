use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use mechanics_http_client::Client as HttpClient;
use serde::{Deserialize, Serialize};

use crate::{config::Config, error::AppError, mint, static_assets};

#[derive(Clone)]
pub(crate) struct AppState {
    config: Config,
    client: HttpClient,
}

impl AppState {
    pub(crate) fn new(config: Config, client: HttpClient) -> Self {
        Self { config, client }
    }
}

pub(crate) fn router(state: AppState) -> Router {
    let config = state.config.clone();
    Router::new()
        .route("/config", get(config_handler))
        .route("/sign-in", post(sign_in))
        .route("/mint-ephemeral", post(mint_ephemeral))
        .route("/version", get(version))
        .fallback(static_assets::serve)
        .with_state(state)
        .layer(axum::Extension(config))
}

#[derive(Serialize)]
struct ConfigResponse {
    api_url: String,
    notify_instance_uuid: String,
}

async fn config_handler(State(state): State<AppState>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        api_url: state.config.chat.api_url.clone(),
        notify_instance_uuid: state.config.chat.notify_instance_uuid.to_string(),
    })
}

#[derive(Deserialize)]
struct SignInRequest {
    agent_token: String,
}

async fn sign_in(
    State(state): State<AppState>,
    Json(request): Json<SignInRequest>,
) -> Result<StatusCode, AppError> {
    if constant_time_eq(
        request.agent_token.as_bytes(),
        state.config.chat.agent_token.as_bytes(),
    ) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::unauthorized())
    }
}

async fn mint_ephemeral(
    State(state): State<AppState>,
) -> Result<Json<mint::MintEphemeralResponse>, AppError> {
    mint::mint_ephemeral(&state.config.chat, &state.client)
        .await
        .map(Json)
}

#[derive(Serialize)]
struct VersionResponse {
    version: &'static str,
    git_commit_sha: Option<&'static str>,
    virtualization: &'static str,
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        git_commit_sha: crate::GIT_COMMIT_SHA,
        virtualization: "unknown",
    })
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn constant_time_eq_matches_equal_bytes() {
        assert!(constant_time_eq(b"pht_secret", b"pht_secret"));
        assert!(!constant_time_eq(b"pht_secret", b"pht_other"));
        assert!(!constant_time_eq(b"pht_secret", b"pht_secret2"));
    }
}
