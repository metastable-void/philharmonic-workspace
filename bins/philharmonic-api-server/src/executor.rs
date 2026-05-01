use async_trait::async_trait;
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
}

#[async_trait]
impl StepExecutor for MechanicsWorkerExecutor {
    async fn execute(
        &self,
        script: &str,
        arg: &JsonValue,
        config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError> {
        let job = json!({
            "module_source": script,
            "arg": arg,
            "config": config,
        });

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

        response.json::<JsonValue>().await.map_err(|error| {
            StepExecutionError::Transport(format!(
                "mechanics worker returned invalid JSON response: {error}"
            ))
        })
    }
}
