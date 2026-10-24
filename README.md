# ProProxy

### Authenticated Reverse Proxy & Cloudflare Tunnel for ProPresenter 7

**ProProxy** (`pro-proxy`) is an ultra-lightweight, standalone macOS application written in Rust. It securely exposes ProPresenter 7's native Web Messages interface (`http://127.0.0.1:1025/html/pages/messages`) to authorized users over the internet via **Cloudflare Tunnel (`cloudflared`)**, enforced by strict route whitelisting and **Argon2id** cryptographic authentication.

> [!IMPORTANT]
> **No Pre-compiled Binaries on GitHub**: Pre-compiled binary files are intentionally excluded from the Git repository. Before deploying to your target machine, you must compile the binary from source using the instructions below. The compilation produces a **Universal macOS Binary** (`aarch64` + `x86_64`) that runs natively on both Apple Silicon (M1/M2/M3/M4) and Intel Macs on macOS 15 Sequoia.

> [!NOTE]
> **AI-Assisted Development**: This project was developed with AI assistance using Google DeepMind's **Gemini 3.8 Flash** model via the Antigravity pair-programming agent environment.

---

## 1. Building the Binary (First Step)

### Prerequisites (on the build machine)
- **Rust Toolchain**: Install Rust if not already present:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **[Cloudflare Tunnel CLI (`cloudflared`)](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)** (installable via [Homebrew](https://brew.sh)):
  ```bash
  brew install cloudflared
  ```

### Option A: Build the Universal Binary (Recommended)
This compiles both `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel) architectures and packages them into a single Mach-O Universal binary:

```bash
# Run the automated universal build script:
./build_universal.sh
```

The script produces the following standalone executables inside the `dist/` directory:
- `dist/pro-proxy`: **Universal macOS Binary** (runs on both Apple Silicon and Intel Macs).
- `dist/pro-proxy-arm64`: Native Apple Silicon binary.
- `dist/pro-proxy-x86_64`: Native Intel Mac binary.

### Option B: Build for Current Architecture Only
```bash
cargo build --release
# Binary will be at: target/release/pro-proxy
```

---

## 2. Deployment & Usage

Once built, `dist/pro-proxy` is a **100% self-contained executable**. It embeds the web server, SQLite engine, Argon2 encryption, and HTML UI. It does **NOT** require Rust, Cargo, or the source code on the target machine.

### Requirement: Cloudflare Tunnel (`cloudflared`)
To securely expose the application to the internet, **[Cloudflare Tunnel](https://www.cloudflare.com/products/tunnel/) (`cloudflared`) is required** on the machine running `pro-proxy`.

- **Recommended Installation (via [Homebrew](https://brew.sh))**:
  ```bash
  brew install cloudflared
  ```
- *Alternative (Manual standalone download without Homebrew)*:
  Download the standalone binary from [Cloudflare's Official Releases](https://github.com/cloudflare/cloudflared/releases) and place it in the same directory as `pro-proxy` or in `/usr/local/bin/cloudflared`.

> [!TIP]
> **macOS Notification Permissions & Alert Style**: On macOS 15 Sequoia, ensure ProPresenter has notifications enabled in **System Settings &rarr; Notifications &rarr; ProPresenter**. It is strongly recommended to set the alert style to **"Alerts"** (instead of "Banners"). Alerts remain visible on screen until dismissed and provide actionable buttons right from the notification popup, allowing the operator to review and approve incoming message requests immediately.

### A. Deploy to Target Mac
Copy the compiled binary (`dist/pro-proxy`) to the target machine (e.g. to `/usr/local/bin/pro-proxy` or `~/pro-proxy`). Ensure `cloudflared` is installed.

### B. Create Admin Account
User accounts are managed only by the administrator via CLI. Create your initial admin user:
```bash
./dist/pro-proxy --set-user admin --password "your_secure_password"
```

### C. Run the Server

#### 1. Default Mode (with Cloudflare Quick Tunnel)
Spawns the local reverse proxy and automatically generates a public `https://*.trycloudflare.com` URL:
```bash
./dist/pro-proxy
```
Look for the output banner in your terminal:
```text
============================================================
  pro-proxy is LIVE on the Internet!
  Public URL: https://example-subdomain.trycloudflare.com
============================================================
```

#### 2. Local Proxy Only (No Tunnel)
If you only want local testing or you manage your own tunnel separately:
```bash
./dist/pro-proxy --no-tunnel --port 8080
```

#### 3. Named Tunnel Mode (Production Custom Domain)
Save your Cloudflare Tunnel token once:
```bash
./dist/pro-proxy --set-token "EYJh...YOUR_CLOUDFLARE_TUNNEL_TOKEN"
./dist/pro-proxy
```

### D. Auto-Start on macOS Boot (LaunchAgent)
To have pro-proxy automatically start whenever the Mac boots or logs in:
```bash
# Register and start the background service
./dist/pro-proxy --install-agent

# To stop and remove the service
./dist/pro-proxy --uninstall-agent
```
Logs are automatically written to `~/Library/Logs/pro-proxy.log`.

---

## 3. CLI Reference

| Flag | Description |
|---|---|
| `-p, --port <PORT>` | Listening port for reverse proxy (default: `8080`) |
| `--pro-port <PORT>` | ProPresenter 7 port (default: `1025`) |
| `--db-path <PATH>` | Custom SQLite database path (default: `~/.pro-proxy/pro-proxy.db`) |
| `--set-user <USER> --password <PASS>` | Create or update a user account |
| `--list-users` | List all registered user accounts |
| `--delete-user <USER>` | Remove a user account |
| `--set-token <TOKEN>` | Store Cloudflare Tunnel token for named tunnels |
| `--no-tunnel` | Run local proxy only without spawning `cloudflared` |
| `--install-agent` | Install and load macOS `LaunchAgent` plist for auto-start |
| `--uninstall-agent` | Unload and remove macOS `LaunchAgent` plist |
| `--status` | Check local ProPresenter port 1025 status and config |

---

## 4. Key Architectural & Security Rules

1. **Strict Route Whitelisting (Port 1025)**:
   - `GET /`: Automatically redirects authenticated users (`302`) to `/html/pages/messages`.
   - `GET /html/pages/messages`: ProPresenter's native web messages interface.
   - `GET /html/assets/*`: All CSS, JS, fonts, and images required by the page.
   - `GET /v1/messages`: Fetches configured messages list.
   - `POST /v1/message/:id/trigger`: Sends message tokens (e.g. `Responsável` and `Criança`).
   - `GET /v1/message/:id/clear`: Clears/hides the active message on screen.
   - **Strict Blocking (`403 Forbidden`)**: Any request to non-whitelisted routes (e.g. `/v1/control`, slide triggers, presentation triggers, stage screens) is rejected immediately.
2. **Argon2id Hashing & Sessions**:
   - Passwords stored as irreversible Argon2id hashes with unique cryptographic salts.
   - Sessions issued via signed `HttpOnly`, `SameSite=Lax`, `Secure` cookies.
   - Rate limiting on `/login` to protect against brute-force attacks.
3. **Standby Auto-Recovery**:
   - If ProPresenter 7 is closed or restarting on port 1025, authenticated users see a clean standby screen with an automatic 3-second retry loop instead of a 502/503 error.
4. **Minimal Footprint**:
   - Idle memory usage: **~8.8 MB RAM** (well below the 20 MB target).
   - Idle CPU usage: **0.0%**.

---

## 5. Automated Verification & Testing

An end-to-end integration test suite is included to verify the proxy against a running ProPresenter 7 instance:

```bash
cargo test --test integration_test -- --nocapture
```

The test suite validates:
1. **Authentication Boundary**: Unauthenticated requests to `/` and `/html/pages/messages` redirect (`302`) to `/login`, and API requests return `401 Unauthorized`.
2. **Security Whitelist**: Blocked routes (`/v1/control`, `/v1/library`, `/v1/looks`) return `403 Forbidden`.
3. **Standby Resilience**: Standby page renders properly when the ProPresenter port is unreachable.
4. **End-to-End Message Submission**: Submits the two-token template message (`Responsável` + `Criança`) through the authenticated proxy to ProPresenter 7, verifies `204 No Content`, and clears it afterwards.
