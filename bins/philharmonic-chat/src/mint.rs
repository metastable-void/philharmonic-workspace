use mechanics_http_client::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{config::ChatConfig, error::AppError};

const RESPONSE_CAP_BYTES: usize = 64 * 1024;
const EPHEMERAL_LIFETIME_SECONDS: u64 = 3600;
const PERMISSION_INSTANCE_EXECUTE: &str = "workflow:instance_execute";
const PERMISSION_INSTANCE_READ: &str = "workflow:instance_read";

#[derive(Debug, Serialize)]
struct CreateInstanceRequest {
    template_id: Uuid,
    args: Value,
}

#[derive(Debug, Deserialize)]
struct CreateInstanceResponse {
    instance_id: Uuid,
}

#[derive(Debug, Serialize)]
struct MintTokenRequest {
    requested_permissions: Vec<&'static str>,
    lifetime_seconds: u64,
    subject: String,
    instance_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct MintTokenResponse {
    token: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MintEphemeralResponse {
    pub(crate) ephemeral_token: String,
    pub(crate) instance_id: Uuid,
}

pub(crate) async fn mint_ephemeral(
    config: &ChatConfig,
    client: &HttpClient,
) -> Result<MintEphemeralResponse, AppError> {
    let instance = create_instance(config, client).await?;
    let token = mint_token(config, client, instance.instance_id).await?;
    Ok(MintEphemeralResponse {
        ephemeral_token: token.token,
        instance_id: instance.instance_id,
    })
}

async fn create_instance(
    config: &ChatConfig,
    client: &HttpClient,
) -> Result<CreateInstanceResponse, AppError> {
    let response = client
        .post(config.api_endpoint("/v1/workflows/instances"))
        .bearer_auth(&config.service_token)
        .json(&CreateInstanceRequest {
            template_id: config.chat_uuid,
            args: json!({}),
        })
        .send()
        .await
        .map_err(|error| {
            AppError::bad_gateway(format!("instance create request failed: {error}"))
        })?;

    decode_api_response(response, "instance create").await
}

async fn mint_token(
    config: &ChatConfig,
    client: &HttpClient,
    instance_id: Uuid,
) -> Result<MintTokenResponse, AppError> {
    let response = client
        .post(config.api_endpoint("/v1/tokens/mint"))
        .bearer_auth(&config.minting_token)
        .json(&MintTokenRequest {
            requested_permissions: vec![PERMISSION_INSTANCE_EXECUTE, PERMISSION_INSTANCE_READ],
            lifetime_seconds: EPHEMERAL_LIFETIME_SECONDS,
            subject: format!("chat-end-user-{}", Uuid::new_v4()),
            instance_id,
        })
        .send()
        .await
        .map_err(|error| AppError::bad_gateway(format!("token mint request failed: {error}")))?;

    decode_api_response(response, "token mint").await
}

async fn decode_api_response<T: for<'de> Deserialize<'de>>(
    response: Response,
    operation: &'static str,
) -> Result<T, AppError> {
    let status = response.status();
    let bytes = response
        .bytes_with_cap(RESPONSE_CAP_BYTES)
        .await
        .map_err(|error| {
            AppError::bad_gateway(format!("{operation} response read failed: {error}"))
        })?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        return Err(AppError::bad_gateway(format!(
            "{operation} failed with API status {status}: {body}"
        )));
    }

    serde_json::from_slice(&bytes).map_err(|error| {
        AppError::bad_gateway(format!("{operation} returned invalid JSON: {error}"))
    })
}
