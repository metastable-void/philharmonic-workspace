use std::{
    fs,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use mechanics_http_client::{Client, StatusCode};
use tokio::time::{Instant, sleep};

#[tokio::test]
#[ignore = "boots mechanics-worker and binds local TCP ports"]
async fn metrics_endpoint_serves_prometheus_text_when_bind_metrics_set() {
    let main_addr = free_addr();
    let metrics_addr = free_addr();
    let config = format!(
        r#"bind = "{main_addr}"
bind_metrics = "{metrics_addr}"
tokens = []

[pool]
execution_timeout_secs = 60
run_timeout_secs = 60
"#
    );
    let mut worker = WorkerProcess::spawn("enabled", &config);
    wait_for_main_listener(&mut worker, main_addr).await;

    let (status, content_type, body) = wait_for_metrics(metrics_addr).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        content_type.starts_with("text/plain; version=0.0.4"),
        "unexpected prometheus content-type: {content_type}"
    );
    assert!(body.contains("# HELP mechanics_pool_workers_total"));
    assert!(body.contains("mechanics_pool_workers_total"));
}

#[tokio::test]
#[ignore = "boots mechanics-worker and binds local TCP ports"]
async fn omitted_bind_metrics_does_not_start_metrics_listener() {
    let main_addr = free_addr();
    let metrics_addr = free_addr();
    let config = format!(
        r#"bind = "{main_addr}"
tokens = []

[pool]
execution_timeout_secs = 60
run_timeout_secs = 60
"#
    );
    let mut worker = WorkerProcess::spawn("disabled", &config);
    wait_for_main_listener(&mut worker, main_addr).await;

    let client = test_client();
    let main_metrics_response = client
        .get(format!("http://{main_addr}/metrics"))
        .send()
        .await
        .expect("main listener should answer /metrics");
    assert_eq!(main_metrics_response.status(), StatusCode::NOT_FOUND);

    let metrics_result = client
        .get(format!("http://{metrics_addr}/metrics"))
        .send()
        .await;
    assert!(
        metrics_result.is_err(),
        "omitted bind_metrics unexpectedly served /metrics"
    );
}

struct WorkerProcess {
    child: Child,
    root: PathBuf,
}

impl WorkerProcess {
    fn spawn(name: &str, config: &str) -> Self {
        let root = temp_root(name);
        let config_dir = root.join("mechanics.toml.d");
        fs::create_dir_all(&config_dir).expect("create config drop-in dir");
        let config_path = root.join("mechanics.toml");
        fs::write(&config_path, config).expect("write config");

        let child = Command::new(env!("CARGO_BIN_EXE_mechanics-worker"))
            .arg("serve")
            .arg("--config")
            .arg(&config_path)
            .arg("--config-dir")
            .arg(&config_dir)
            .spawn()
            .expect("spawn mechanics-worker");

        Self { child, root }
    }

    fn assert_running(&mut self) {
        if let Some(status) = self.child.try_wait().expect("poll mechanics-worker") {
            panic!("mechanics-worker exited early: {status}")
        }
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

async fn wait_for_main_listener(worker: &mut WorkerProcess, addr: SocketAddr) {
    let client = test_client();
    let url = format!("http://{addr}/");
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        worker.assert_running();
        match client.get(&url).send().await {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::NOT_FOUND);
                return;
            }
            Err(_) if Instant::now() < deadline => {
                sleep(Duration::from_millis(50)).await;
            }
            Err(error) => panic!("main listener did not become ready: {error}"),
        }
    }
}

async fn wait_for_metrics(addr: SocketAddr) -> (StatusCode, String, String) {
    let client = test_client();
    let url = format!("http://{addr}/metrics");
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let body = response.text().await.expect("read metrics body");
                return (status, content_type, body);
            }
            Err(_) if Instant::now() < deadline => {
                sleep(Duration::from_millis(50)).await;
            }
            Err(error) => panic!("metrics listener did not become ready: {error}"),
        }
    }
}

fn test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_millis(250))
        .build()
        .expect("build HTTP client")
}

fn free_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free port probe");
    listener.local_addr().expect("read free port probe addr")
}

fn temp_root(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "mechanics-worker-metrics-{name}-{}-{}",
        std::process::id(),
        unique_nanos()
    ));
    root
}

fn unique_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos()
}
