use std::path::PathBuf;
use std::process::Command;

const AGENT_LABEL: &str = "com.proproxy.agent";

pub fn get_agent_plist_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{AGENT_LABEL}.plist")))
}

pub fn install_launch_agent() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {e}"))?;

    let plist_path = get_agent_plist_path()?;
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create LaunchAgents directory: {e}"))?;
    }

    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let log_out = format!("{home}/Library/Logs/pro-proxy.log");
    let log_err = format!("{home}/Library/Logs/pro-proxy.err.log");

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
</dict>
</plist>"#,
        current_exe.display(),
        log_out,
        log_err
    );

    std::fs::write(&plist_path, plist_content)
        .map_err(|e| format!("Failed to write plist file: {e}"))?;

    // Load agent with launchctl
    let output = Command::new("launchctl")
        .args(["load", "-w", plist_path.to_str().unwrap()])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!("LaunchAgent installed and loaded successfully at: {}", plist_path.display());
            Ok(())
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            println!("LaunchAgent plist written to {}, but launchctl returned: {}", plist_path.display(), err);
            Ok(())
        }
        Err(e) => {
            println!("LaunchAgent plist written to {}, but launchctl command failed: {}", plist_path.display(), e);
            Ok(())
        }
    }
}

pub fn uninstall_launch_agent() -> Result<(), String> {
    let plist_path = get_agent_plist_path()?;
    if !plist_path.exists() {
        println!("LaunchAgent plist not found at {}", plist_path.display());
        return Ok(());
    }

    let _ = Command::new("launchctl")
        .args(["unload", "-w", plist_path.to_str().unwrap()])
        .output();

    std::fs::remove_file(&plist_path)
        .map_err(|e| format!("Failed to delete plist file: {e}"))?;

    println!("LaunchAgent uninstalled successfully.");
    Ok(())
}
