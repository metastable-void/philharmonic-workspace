use async_trait::async_trait;
use std::time::Duration;

use philharmonic::types::JsonValue;
use philharmonic::workflow::{StepExecutionError, StepExecutor};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;

pub(crate) struct MechanicsWorkerExecutor {
    client: reqwest::Client,
    worker_url: String,
    bearer_token: Option<String>,
}

impl MechanicsWorkerExecutor {
    pub(crate) fn new(worker_url: String, token: Option<String>) -> Result<Self, String> {
        let worker_url = worker_url.trim().trim_end_matches('/').to_owned();
        if worker_url.is_empty() {
            return Err("mechanics worker URL must not be empty".to_string());
        }
        reqwest::Url::parse(&worker_url)
            .map_err(|error| format!("mechanics worker URL is invalid: {error}"))?;
        Ok(Self {
            client: reqwest::Client::new(),
            worker_url,
            bearer_token: token,
        })
    }

    pub(crate) async fn execute_with_run_timeout(
        &self,
        script: &str,
        arg: &JsonValue,
        config: &JsonValue,
        run_timeout: Duration,
        max_response_bytes: Option<usize>,
    ) -> Result<JsonValue, StepExecutionError> {
        self.execute_job(script, arg, config, Some(run_timeout), max_response_bytes)
            .await
    }

    async fn execute_job(
        &self,
        script: &str,
        arg: &JsonValue,
        config: &JsonValue,
        run_timeout: Option<Duration>,
        max_response_bytes: Option<usize>,
    ) -> Result<JsonValue, StepExecutionError> {
        let mut job = json!({
            "module_source": script,
            "arg": arg,
            "config": config,
        });
        if let Some(timeout) = run_timeout {
            job["run_timeout"] = json!({
                "secs": timeout.as_secs(),
                "nanos": timeout.subsec_nanos(),
            });
        }

        let url = format!("{}/api/v1/mechanics", self.worker_url);
        let mut request = self
            .client
            .post(&url)
            .header(CONTENT_TYPE, "application/json");
        if let Some(token) = &self.bearer_token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = request.json(&job).send().await.map_err(|error| {
            StepExecutionError::Transport(format!("mechanics worker request failed: {error}"))
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) => format!("<failed to read error body: {error}>"),
            };
            let detail = format!("mechanics worker returned status {status}: {body}");
            return if status.is_server_error() {
                Err(StepExecutionError::Transport(detail))
            } else {
                Err(StepExecutionError::ScriptError(detail))
            };
        }

        let body_bytes = match max_response_bytes {
            Some(max) => read_capped_body(response, max).await?,
            None => response
                .bytes()
                .await
                .map_err(|error| {
                    StepExecutionError::Transport(format!(
                        "mechanics worker response body read failed: {error}"
                    ))
                })?
                .to_vec(),
        };

        serde_json::from_slice(&body_bytes).map_err(|error| {
            StepExecutionError::Transport(format!(
                "mechanics worker returned invalid JSON response: {error}"
            ))
        })
    }
}

/// Streaming-read a reqwest response body up to `max_bytes`. Errors with
/// a `Transport` variant if the cumulative read exceeds the cap. Used by
/// the embed-job path (per Gate-2 pre-review Finding 1.5) to bound
/// transient JSON-in-memory pressure before parsing.
async fn read_capped_body(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, StepExecutionError> {
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        StepExecutionError::Transport(format!("mechanics worker chunk read failed: {error}"))
    })? {
        if buf.len().saturating_add(chunk.len()) > max_bytes {
            return Err(StepExecutionError::Transport(format!(
                "mechanics worker response exceeds cap of {max_bytes} bytes \
                 (read {} bytes before next chunk of {} bytes pushed past)",
                buf.len(),
                chunk.len()
            )));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

#[async_trait]
impl StepExecutor for MechanicsWorkerExecutor {
    async fn execute(
        &self,
        script: &str,
        arg: &JsonValue,
        config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError> {
        self.execute_job(script, arg, config, None, None).await
    }
}
