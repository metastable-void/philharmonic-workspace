use async_trait::async_trait;
use philharmonic::types::JsonValue;
use philharmonic::workflow::{StepExecutionError, StepExecutor};
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub(crate) struct MechanicsWorkerExecutor {
    client: reqwest::Client,
    worker_url: String,
}

impl MechanicsWorkerExecutor {
    pub(crate) fn new(worker_url: String) -> Result<Self, String> {
        let worker_url = worker_url.trim().to_owned();
        if worker_url.is_empty() {
            return Err("mechanics worker URL must not be empty".to_string());
        }
        reqwest::Url::parse(&worker_url)
            .map_err(|error| format!("mechanics worker URL is invalid: {error}"))?;
        Ok(Self {
            client: reqwest::Client::new(),
            worker_url,
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

        let response = self
            .client
            .post(&self.worker_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&job)
            .send()
            .await
            .map_err(|error| {
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
