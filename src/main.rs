mod auth;
mod cli;
mod db;
mod proxy;
mod service;
mod tunnel;
mod ui;

use clap::Parser;
use cli::Cli;
use db::{default_db_path, Db};
use proxy::{create_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,pro_proxy=debug".into()),
        )
        .init();

    let cli = Cli::parse();
    let db_path = cli.db_path.unwrap_or_else(default_db_path);
    let db = Db::open(&db_path)?;

    // Handle LaunchAgent commands
    if cli.install_agent {
        service::install_launch_agent().map_err(|e| Box::<dyn std::error::Error>::from(e))?;
        return Ok(());
    }

    if cli.uninstall_agent {
        service::uninstall_launch_agent().map_err(|e| Box::<dyn std::error::Error>::from(e))?;
        return Ok(());
    }

    // Handle Token configuration
    if let Some(token) = cli.set_token {
        db.set_config("tunnel_token", &token)?;
        println!("Cloudflare Tunnel token saved successfully.");
        return Ok(());
    }

    // Handle User management commands
    if cli.list_users {
        let users = db.list_users()?;
        if users.is_empty() {
            println!("No users registered. Add one using: pro-proxy --set-user <username> --password <password>");
        } else {
            println!("Registered users ({}):", users.len());
            for user in users {
                println!(" - {}", user);
            }
        }
        return Ok(());
    }

    if let Some(username) = cli.delete_user {
        let deleted = db.delete_user(&username)?;
        if deleted {
            println!("User '{}' successfully deleted.", username);
        } else {
            println!("User '{}' not found.", username);
        }
        return Ok(());
    }

    if let Some(username) = cli.set_user {
        let password = if let Some(p) = cli.password {
            p
        } else {
            eprintln!("Please specify a password via --password <password>");
            std::process::exit(1);
        };

        if password.trim().is_empty() {
            eprintln!("Password cannot be empty.");
            std::process::exit(1);
        }

        let hash = auth::hash_password(&password).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
        db.set_user(&username, &hash)?;
        println!("User '{}' created/updated successfully.", username);
        return Ok(());
    }

    // Handle status command
    if cli.status {
        print_status(&db, cli.pro_port).await;
        return Ok(());
    }

    // Server mode
    let user_count = db.user_count()?;
    if user_count == 0 {
        println!();
        println!("============================================================");
        println!("  WARNING: No users are registered in the database!");
        println!("  Please create an admin user with:");
        println!("    ./pro-proxy --set-user admin --password <your_password>");
        println!("============================================================");
        println!();
    }

    let tunnel_manager = Arc::new(tunnel::TunnelManager::new());
    let mut tunnel_child = None;

    if !cli.no_tunnel {
        let saved_token = db.get_config("tunnel_token")?;
        match tunnel_manager.start(cli.port, saved_token).await {
            Ok(child) => {
                tunnel_child = Some(child);
            }
            Err(e) => {
                warn!("Cloudflare Tunnel could not be started: {e}");
                warn!("Continuing with local proxy only on http://127.0.0.1:{}", cli.port);
            }
        }
    }

    let state = AppState::new(db.clone(), cli.pro_port);
    let app = create_router(state);

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    info!("pro-proxy listening on http://{}", bind_addr);
    info!("Target ProPresenter backend: http://127.0.0.1:{}", cli.pro_port);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    tokio::select! {
        res = server => {
            if let Err(e) = res {
                error!("Server error: {e}");
            }
        }
        _ = shutdown_signal() => {
            info!("Received shutdown signal. Stopping pro-proxy...");
        }
    }

    if let Some(mut child) = tunnel_child {
        info!("Stopping Cloudflare Tunnel...");
        let _ = child.kill().await;
    }

    // Clean expired sessions on exit
    let _ = db.cleanup_expired_sessions();
    info!("Shutdown complete.");
    Ok(())
}

async fn print_status(db: &Db, pro_port: u16) {
    println!("=== pro-proxy Status ===");
    let users = db.list_users().unwrap_or_default();
    println!("Database users: {} registered", users.len());
    for u in &users {
        println!("  - {}", u);
    }

    let token = db.get_config("tunnel_token").unwrap_or(None);
    println!("Custom tunnel token: {}", if token.is_some() { "Configured" } else { "None (Quick Tunnel mode)" });

    print!("ProPresenter (127.0.0.1:{pro_port}): ");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    match client.get(format!("http://127.0.0.1:{pro_port}/version")).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            println!("ONLINE ({body})");
        }
        Ok(resp) => println!("RESPONDING with status {}", resp.status()),
        Err(e) => println!("OFFLINE ({e})"),
    }

    if let Ok(path) = service::get_agent_plist_path() {
        println!("LaunchAgent plist: {}", if path.exists() { "Installed" } else { "Not installed" });
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
