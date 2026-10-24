use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "pro-proxy",
    version,
    about = "pro-proxy: Authenticated Reverse Proxy for ProPresenter 7"
)]
pub struct Cli {
    /// Port for the reverse proxy to listen on
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// ProPresenter port
    #[arg(long, default_value_t = 1025)]
    pub pro_port: u16,

    /// Custom path to SQLite database
    #[arg(long)]
    pub db_path: Option<PathBuf>,

    /// Set or update a user
    #[arg(long, value_name = "USERNAME")]
    pub set_user: Option<String>,

    /// Password when using --set-user
    #[arg(long, value_name = "PASSWORD")]
    pub password: Option<String>,

    /// List all registered users
    #[arg(long)]
    pub list_users: bool,

    /// Delete a user
    #[arg(long, value_name = "USERNAME")]
    pub delete_user: Option<String>,

    /// Set Cloudflare Tunnel token
    #[arg(long, value_name = "TOKEN")]
    pub set_token: Option<String>,

    /// Do not start Cloudflare tunnel (run proxy only)
    #[arg(long)]
    pub no_tunnel: bool,

    /// Install macOS LaunchAgent for auto-start
    #[arg(long)]
    pub install_agent: bool,

    /// Uninstall macOS LaunchAgent
    #[arg(long)]
    pub uninstall_agent: bool,

    /// Check ProPresenter status and pro-proxy config
    #[arg(long)]
    pub status: bool,
}
