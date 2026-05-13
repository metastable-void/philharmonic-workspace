use async_trait::async_trait;
use std::str::FromStr;
use std::time::Duration;

use mechanics_http_client::Uri;
use philharmonic::types::JsonValue;
use philharmonic::workflow::{StepExecutionError, StepExecutor};
use serde_json::json;

pub(crate) struct MechanicsWorkerExecutor {
    client: mechanics_http_client::Client,
    worker_url: String,
    bearer_token: Option<String>,
}

impl MechanicsWorkerExecutor {
    pub(crate) fn new(worker_url: String, token: Option<String>) -> Result<Self, String> {
        let worker_url = worker_url.trim().trim_end_matches('/').to_owned();
        if worker_url.is_empty() {
            return Err("mechanics worker URL must not be empty".to_string());
        }
        Uri::from_str(&worker_url)
            .map_err(|error| format!("mechanics worker URL is invalid: {error}"))?;
        let client = mechanics_http_client::Client::new()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;
        Ok(Self {
            client,
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
            .header("content-type", "application/json");
        if let Some(token) = &self.bearer_token {
            request = request.bearer_auth(token);
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

/// Read the response body up to `max_bytes` of *wire* bytes, then
/// decompress per `Content-Encoding`. Errors with `Transport` if the
/// cap is exceeded. Used by the embed-job path (per Gate-2 pre-review
/// Finding 1.5) to bound transient JSON-in-memory pressure before
/// parsing.
async fn read_capped_body(
    response: mechanics_http_client::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, StepExecutionError> {
    match response.bytes_with_cap(max_bytes).await {
        Ok(bytes) => Ok(bytes.to_vec()),
        Err(mechanics_http_client::Error::BodyTooLarge { limit, seen }) => {
            Err(StepExecutionError::Transport(format!(
                "mechanics worker response exceeds cap of {limit} bytes \
                 (read {seen} bytes before next chunk pushed past)"
            )))
        }
        Err(other) => Err(StepExecutionError::Transport(format!(
            "mechanics worker response body read failed: {other}"
        ))),
    }
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
