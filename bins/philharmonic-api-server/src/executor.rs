use async_trait::async_trait;
use philharmonic::types::JsonValue;
use philharmonic::workflow::{StepExecutionError, StepExecutor};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

const ENCRYPTED_PAYLOAD_HEADER: &str = "X-Encrypted-Payload";

pub struct HttpStepExecutor {
    client: reqwest::Client,
    connector_url: String,
}

impl HttpStepExecutor {
    pub fn new(connector_url: String) -> Result<Self, String> {
        let connector_url = connector_url.trim().to_owned();
        if connector_url.is_empty() {
            return Err("connector service URL must not be empty".to_string());
        }
        reqwest::Url::parse(&connector_url)
            .map_err(|error| format!("connector service URL is invalid: {error}"))?;
        Ok(Self {
            client: reqwest::Client::new(),
            connector_url,
        })
    }
}

#[async_trait]
impl StepExecutor for HttpStepExecutor {
    async fn execute(
        &self,
        _script: &str,
        arg: &JsonValue,
        config: &JsonValue,
    ) -> Result<JsonValue, StepExecutionError> {
        let token = required_string(config, "token")?;
        let encrypted_payload = required_string(config, "encrypted_payload")?;
        let request_body = config
            .get("request")
            .cloned()
            .unwrap_or_else(|| arg.clone());

        let response = self
            .client
            .post(&self.connector_url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(ENCRYPTED_PAYLOAD_HEADER, encrypted_payload)
            .header(CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|error| {
                StepExecutionError::Transport(format!("connector service request failed: {error}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) => format!("<failed to read error body: {error}>"),
            };
            let detail = format!("connector service returned status {status}: {body}");
            return if status.is_server_error() {
                Err(StepExecutionError::Transport(detail))
            } else {
                Err(StepExecutionError::ScriptError(detail))
            };
        }

        response.json::<JsonValue>().await.map_err(|error| {
            StepExecutionError::Transport(format!(
                "connector service returned invalid JSON response: {error}"
            ))
        })
    }
}

fn required_string<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a str, StepExecutionError> {
    value.get(field).and_then(JsonValue::as_str).ok_or_else(|| {
        StepExecutionError::ScriptError(format!("lowered config field '{field}' must be a string"))
    })
}
