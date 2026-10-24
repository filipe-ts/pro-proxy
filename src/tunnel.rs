use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone)]
pub struct TunnelManager {
    pub current_url: Arc<RwLock<Option<String>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            current_url: Arc::new(RwLock::new(None)),
        }
    }

    pub fn find_cloudflared() -> Option<PathBuf> {
        let mut candidates = Vec::new();

        // 1. Next to current executable
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                candidates.push(parent.join("cloudflared"));
            }
        }

        // 2. In current working directory
        candidates.push(PathBuf::from("./cloudflared"));

        // 3. System locations
        candidates.push(PathBuf::from("cloudflared"));
        candidates.push(PathBuf::from("/usr/local/bin/cloudflared"));
        candidates.push(PathBuf::from("/opt/homebrew/bin/cloudflared"));
        candidates.push(PathBuf::from("/usr/bin/cloudflared"));

        for candidate in candidates {
            if let Ok(output) = std::process::Command::new(&candidate).arg("--version").output() {
                if output.status.success() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub async fn start(
        &self,
        local_port: u16,
        token: Option<String>,
    ) -> Result<Child, String> {
        let bin_path = Self::find_cloudflared()
            .ok_or_else(|| "cloudflared binary not found in PATH or standard macOS directories. Install with: brew install cloudflared".to_string())?;

        let mut cmd = Command::new(&bin_path);

        if let Some(tok) = token {
            info!("Starting Cloudflare Tunnel with token...");
            cmd.args(["tunnel", "run", "--token", &tok]);
        } else {
            info!("Starting Cloudflare Quick Tunnel for port {local_port}...");
            cmd.args([
                "tunnel",
                "--url",
                &format!("http://127.0.0.1:{local_port}"),
                "--no-autoupdate",
            ]);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn cloudflared: {e}"))?;

        let stderr = child.stderr.take().ok_or("Failed to capture cloudflared stderr")?;
        let current_url = self.current_url.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let url_regex = Regex::new(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com").unwrap();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(mat) = url_regex.find(&line) {
                    let url = mat.as_str().to_string();
                    let mut lock = current_url.write().await;
                    if lock.as_deref() != Some(&url) {
                        *lock = Some(url.clone());
                        println!();
                        println!("============================================================");
                        println!("  pro-proxy is LIVE on the Internet!");
                        println!("  Public URL: {}", url);
                        println!("============================================================");
                        println!();
                        info!("Cloudflare Tunnel established: {}", url);
                    }
                } else if line.contains("ERR") || line.contains("error") {
                    warn!("[cloudflared] {}", line);
                }
            }
        });

        Ok(child)
    }
}
