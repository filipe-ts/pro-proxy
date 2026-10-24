pub fn render_login_page(error: Option<&str>) -> String {
    let error_html = match error {
        Some("invalid") => {
            r#"<div class="alert alert-error">Usuário ou senha incorretos. / Invalid username or password.</div>"#
        }
        Some("rate_limited") => {
            r#"<div class="alert alert-error">Muitas tentativas. Aguarde 1 minuto. / Too many attempts. Please wait 1 minute.</div>"#
        }
        Some(_) => r#"<div class="alert alert-error">Erro na autenticação.</div>"#,
        None => "",
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>pro-proxy — Login</title>
  <style>
    :root {{
      --bg: #0f172a;
      --card-bg: #1e293b;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --primary: #3b82f6;
      --primary-hover: #2563eb;
      --border: #334155;
      --error-bg: #ef444422;
      --error-border: #ef4444;
      --error-text: #fca5a5;
    }}
    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }}
    body {{
      background-color: var(--bg);
      color: var(--text);
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      padding: 1rem;
    }}
    .login-card {{
      background-color: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 1rem;
      padding: 2.25rem 2rem;
      width: 100%;
      max-width: 400px;
      box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.4), 0 8px 10px -6px rgba(0, 0, 0, 0.3);
    }}
    .header {{
      text-align: center;
      margin-bottom: 2rem;
    }}
    .badge {{
      display: inline-block;
      padding: 0.35rem 0.75rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 600;
      letter-spacing: 0.05em;
      text-transform: uppercase;
      background-color: rgba(59, 130, 246, 0.15);
      color: #60a5fa;
      border: 1px solid rgba(59, 130, 246, 0.3);
      margin-bottom: 0.85rem;
    }}
    h1 {{
      font-size: 1.5rem;
      font-weight: 700;
      letter-spacing: -0.025em;
    }}
    .subtitle {{
      color: var(--text-muted);
      font-size: 0.875rem;
      margin-top: 0.35rem;
    }}
    .alert-error {{
      background-color: var(--error-bg);
      border: 1px solid var(--error-border);
      color: var(--error-text);
      padding: 0.75rem 1rem;
      border-radius: 0.5rem;
      font-size: 0.875rem;
      margin-bottom: 1.25rem;
      line-height: 1.4;
    }}
    .form-group {{
      margin-bottom: 1.25rem;
    }}
    label {{
      display: block;
      font-size: 0.875rem;
      font-weight: 500;
      margin-bottom: 0.45rem;
      color: var(--text);
    }}
    input[type="text"],
    input[type="password"] {{
      width: 100%;
      padding: 0.75rem 1rem;
      border-radius: 0.5rem;
      border: 1px solid var(--border);
      background-color: rgba(15, 23, 42, 0.6);
      color: #fff;
      font-size: 1rem;
      outline: none;
      transition: border-color 0.15s ease, box-shadow 0.15s ease;
    }}
    input:focus {{
      border-color: var(--primary);
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.3);
    }}
    .btn {{
      width: 100%;
      padding: 0.85rem;
      border-radius: 0.5rem;
      border: none;
      background-color: var(--primary);
      color: #fff;
      font-size: 1rem;
      font-weight: 600;
      cursor: pointer;
      transition: background-color 0.15s ease;
      margin-top: 0.5rem;
    }}
    .btn:hover {{
      background-color: var(--primary-hover);
    }}
    .footer {{
      margin-top: 1.75rem;
      text-align: center;
      font-size: 0.75rem;
      color: var(--text-muted);
    }}
  </style>
</head>
<body>
  <div class="login-card">
    <div class="header">
      <div class="badge">Acesso Seguro</div>
      <h1>ProPresenter 7</h1>
      <p class="subtitle">Notificações e Mensagens da Igreja</p>
    </div>
    {error_html}
    <form method="POST" action="/login">
      <div class="form-group">
        <label for="username">Usuário</label>
        <input type="text" id="username" name="username" required autocomplete="username" autofocus>
      </div>
      <div class="form-group">
        <label for="password">Senha</label>
        <input type="password" id="password" name="password" required autocomplete="current-password">
      </div>
      <button type="submit" class="btn">Entrar</button>
    </form>
    <div class="footer">
      pro-proxy &bull; Tunnel Seguro
    </div>
  </div>
</body>
</html>"#
    )
}

pub fn render_standby_page() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Aguardando ProPresenter 7</title>
  <style>
    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }
    body {
      background-color: #0f172a;
      color: #f8fafc;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
      padding: 1.5rem;
      text-align: center;
    }
    .card {
      background-color: #1e293b;
      border: 1px solid #334155;
      border-radius: 1rem;
      padding: 2.5rem 2rem;
      max-width: 440px;
      box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.4);
    }
    .spinner {
      width: 48px;
      height: 48px;
      border: 4px solid rgba(59, 130, 246, 0.2);
      border-top-color: #3b82f6;
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin: 0 auto 1.5rem auto;
    }
    @keyframes spin {
      to { transform: rotate(360deg); }
    }
    h2 {
      font-size: 1.25rem;
      margin-bottom: 0.75rem;
      color: #f8fafc;
    }
    p {
      color: #94a3b8;
      font-size: 0.9rem;
      line-height: 1.5;
      margin-bottom: 1.25rem;
    }
    .status {
      font-size: 0.8rem;
      color: #64748b;
      padding: 0.5rem;
      background: rgba(15, 23, 42, 0.5);
      border-radius: 0.375rem;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="spinner"></div>
    <h2>Aguardando o ProPresenter 7</h2>
    <p>O ProPresenter 7 não está aberto no computador principal no momento. Assim que ele for iniciado, esta tela se conectará automaticamente.</p>
    <div class="status" id="countdown">Tentando reconectar em 3s...</div>
  </div>
  <script>
    let seconds = 3;
    const el = document.getElementById('countdown');
    setInterval(() => {
      seconds--;
      if (seconds <= 0) {
        window.location.reload();
      } else {
        el.innerText = `Tentando reconectar em ${seconds}s...`;
      }
    }, 1000);
  </script>
</body>
</html>"#
}
