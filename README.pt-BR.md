# ProProxy

[![English](https://img.shields.io/badge/Language-English-blue?style=flat-square)](README.md)
[![Português (Brasil)](https://img.shields.io/badge/Idioma-Português_(Brasil)-green?style=flat-square)](README.pt-BR.md)

### Proxy Reverso Autenticado e Cloudflare Tunnel para ProPresenter 7

**ProProxy** (`pro-proxy`) é uma aplicação macOS autônoma e ultraleve escrita em Rust. Ela expõe com segurança a interface nativa de Mensagens Web do ProPresenter 7 (`http://127.0.0.1:1025/html/pages/messages`) para usuários autorizados através da internet via **Cloudflare Tunnel (`cloudflared`)**, protegida por lista estrita de rotas permitidas (whitelisting) e autenticação criptográfica **Argon2id**.

> [!IMPORTANT]
> **Sem Binários Pré-compilados no Git**: Arquivos binários compilados são intencionalmente excluídos do repositório Git. Antes de implantar na sua máquina de destino, compile o binário a partir do código-fonte utilizando as instruções abaixo. A compilação gera um **Binário Universal para macOS** (`aarch64` + `x86_64`) que roda nativamente tanto em Apple Silicon (M1/M2/M3/M4) quanto em Macs Intel no macOS 15 Sequoia.

> [!NOTE]
> **Desenvolvimento Assistido por IA**: Este projeto foi desenvolvido com assistência de IA utilizando o modelo **Gemini 3.8 Flash** da Google DeepMind através do ambiente de agente de programação em par Antigravity.

---

### Por que este projeto existe

Em muitas igrejas e comunidades onde o ProPresenter é utilizado, a infraestrutura da rede local raramente é configurada para permitir uma comunicação direta e fluida entre dispositivos. Voluntários e equipes de ministério (como coordenadores de berçário, líderes do ministério infantil ou diretores de culto) frequentemente encontram obstáculos de conectividade:
- Os voluntários muitas vezes só têm acesso a uma rede Wi-Fi de visitantes (Guest) que isola os clientes e bloqueia o tráfego local da sub-rede.
- A rede do prédio pode estar dividida em múltiplas sub-redes ou VLANs que não se comunicam entre si.
- Voluntários em salas anexas ou locais distantes podem dispor apenas de dados móveis (3G/4G/5G).

Em condições normais, a interface nativa de Mensagens Web do ProPresenter 7 exige que o usuário esteja conectado exatamente à mesma rede local. Além disso, expor a porta 1025 diretamente para a internet é perigoso, pois ela não possui controle de acesso por senha e expõe toda a API de controle remoto do software. O **ProProxy** resolve esse problema criando um túnel seguro via Cloudflare com autenticação estrita via Argon2id e bloqueio rigoroso de rotas administrativas. Dessa forma, pessoas autorizadas da equipe podem enviar alertas de mensagens de forma confiável pelo celular ou tablet pela internet, sem depender da configuração da rede local do prédio.

---

## 1. Compilando o Binário (Primeiro Passo)

### Pré-requisitos (na máquina de compilação)
- **Rust Toolchain**: Instale o Rust se ainda não o tiver:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **[Cloudflare Tunnel CLI (`cloudflared`)](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)** (instalável via [Homebrew](https://brew.sh)):
  ```bash
  brew install cloudflared
  ```

### Opção A: Compilar o Binário Universal (Recomendado)
Compila para as arquiteturas `aarch64-apple-darwin` (Apple Silicon) e `x86_64-apple-darwin` (Intel) e junta ambas em um único binário Mach-O Universal:

```bash
# Executa o script automatizado de build universal:
./build_universal.sh
```

O script gera os seguintes executáveis independentes dentro do diretório `dist/`:
- `dist/pro-proxy`: **Binário Universal para macOS** (executa nativamente em Apple Silicon e Intel).
- `dist/pro-proxy-arm64`: Binário nativo para Apple Silicon.
- `dist/pro-proxy-x86_64`: Binário nativo para Macs Intel.

### Opção B: Compilar Apenas para a Arquitetura Atual
```bash
cargo build --release
# O binário estará em: target/release/pro-proxy
```

---

## 2. Implantação e Uso

Após compilado, `dist/pro-proxy` é um **executável 100% autocontido**. Ele embute o servidor web, o banco de dados SQLite, a criptografia Argon2 e a interface HTML. Ele **NÃO** requer Rust, Cargo ou o código-fonte na máquina de destino.

### Requisito: Cloudflare Tunnel (`cloudflared`)
Para expor a aplicação com segurança na internet, o **[Cloudflare Tunnel](https://www.cloudflare.com/products/tunnel/) (`cloudflared`) é necessário** na máquina que executará o `pro-proxy`.

- **Instalação Recomendada (via [Homebrew](https://brew.sh))**:
  ```bash
  brew install cloudflared
  ```
- *Alternativa (Download avulso sem Homebrew)*:
  Baixe o binário avulso nos [Releases Oficiais do Cloudflare](https://github.com/cloudflare/cloudflared/releases) e coloque-o na mesma pasta do `pro-proxy` ou em `/usr/local/bin/cloudflared`.

> [!TIP]
> **Permissões de Notificação no macOS e Estilo de Alerta**: No macOS 15 Sequoia, certifique-se de que o ProPresenter esteja com as notificações ativadas em **Ajustes do Sistema &rarr; Notificações &rarr; ProPresenter**. É altamente recomendável configurar o estilo de alerta como **"Alertas"** (em vez de "Banners"). Os alertas permanecem na tela até serem dispensados e exibem botões de ação na própria notificação, permitindo ao operador aprovar ou descartar pedidos de mensagem imediatamente.

### A. Implantar no Mac de Destino
Copie o binário compilado (`dist/pro-proxy`) para a máquina de destino (por exemplo, para `/usr/local/bin/pro-proxy` ou `~/pro-proxy`). Certifique-se de que o `cloudflared` esteja instalado.

### B. Criar Conta de Administrador
As contas de usuário são gerenciadas unicamente pelo administrador via linha de comando. Crie seu usuário inicial:
```bash
./dist/pro-proxy --set-user admin --password "sua_senha_segura"
```

### C. Executar o Servidor

#### 1. Modo Padrão (com Quick Tunnel do Cloudflare)
Inicia o proxy reverso local e gera automaticamente uma URL pública em `https://*.trycloudflare.com`:
```bash
./dist/pro-proxy
```
Observe o banner de saída no seu terminal:
```text
============================================================
  pro-proxy is LIVE on the Internet!
  Public URL: https://example-subdomain.trycloudflare.com
============================================================
```

#### 2. Modo Apenas Proxy Local (Sem Túnel)
Caso queira testar apenas localmente ou gerencie seu próprio túnel separadamente:
```bash
./dist/pro-proxy --no-tunnel --port 8080
```

#### 3. Modo Túnel Nomeado (Domínio Próprio de Produção)
Grave seu token do Cloudflare Tunnel uma única vez:
```bash
./dist/pro-proxy --set-token "EYJh...SEU_TOKEN_DO_CLOUDFLARE_TUNNEL"
./dist/pro-proxy
```

### D. Inicialização Automática no Boot do macOS (LaunchAgent)
Para que o `pro-proxy` inicie automaticamente sempre que o Mac ligar ou fizer login:
```bash
# Registra e inicializa o serviço em segundo plano
./dist/pro-proxy --install-agent

# Para parar e remover o serviço
./dist/pro-proxy --uninstall-agent
```
Os registros de log são gravados automaticamente em `~/Library/Logs/pro-proxy.log`.

---

## 3. Referência da Linha de Comando (CLI)

| Parâmetro | Descrição |
|---|---|
| `-p, --port <PORTA>` | Porta de escuta do proxy reverso (padrão: `8080`) |
| `--pro-port <PORTA>` | Porta do ProPresenter 7 (padrão: `1025`) |
| `--db-path <CAMINHO>` | Caminho personalizado para o banco SQLite (padrão: `~/.pro-proxy/pro-proxy.db`) |
| `--set-user <USUÁRIO> --password <SENHA>` | Cria ou atualiza uma conta de usuário |
| `--list-users` | Lista todas as contas de usuários cadastradas |
| `--delete-user <USUÁRIO>` | Remove uma conta de usuário |
| `--set-token <TOKEN>` | Salva o token do Cloudflare Tunnel para túneis nomeados |
| `--no-tunnel` | Executa o proxy local sem iniciar o `cloudflared` |
| `--install-agent` | Instala e carrega o arquivo plist do `LaunchAgent` no macOS |
| `--uninstall-agent` | Descarrega e remove o arquivo plist do `LaunchAgent` |
| `--status` | Verifica o status da porta 1025 do ProPresenter local e configurações |

---

## 4. Principais Regras de Arquitetura e Segurança

1. **Lista Estrita de Rotas Permitidas (Porta 1025)**:
   - `GET /`: Redireciona automaticamente usuários autenticados (`302`) para `/html/pages/messages`.
   - `GET /html/pages/messages`: Interface nativa de mensagens web do ProPresenter.
   - `GET /html/assets/*`: Todos os arquivos CSS, JS, fontes e imagens necessários para a página.
   - `GET /v1/messages`: Obtém a lista de mensagens configuradas.
   - `POST /v1/message/:id/trigger`: Dispara os tokens da mensagem (ex.: `Guardian` e `Child`).
   - `GET /v1/message/:id/clear`: Limpa/oculta a mensagem ativa da tela.
   - **Bloqueio Estrito (`403 Forbidden`)**: Qualquer requisição para rotas não listadas (ex.: `/v1/control`, disparo de slides, apresentações, telas de retorno) é sumariamente rejeitada.
2. **Criptografia Argon2id e Sessões**:
   - Senhas armazenadas como hashes irreversíveis com Argon2id e salts criptográficos exclusivos.
   - Sessões emitidas através de cookies assinados com flags `HttpOnly`, `SameSite=Lax` e `Secure`.
   - Limitação de taxa de tentativas (rate limiting) no `/login` para proteção contra ataques de força bruta.
3. **Recuperação Automática com Tela de Standby**:
   - Se o ProPresenter 7 estiver fechado ou reiniciando na porta 1025, os usuários autenticados visualizam uma tela amigável de espera com retentativa automática a cada 3 segundos, em vez de um erro 502/503 cru.
4. **Pegada Mínima de Recursos**:
   - Uso de memória em repouso: **~8,8 MB de RAM** (muito abaixo da meta de 20 MB).
   - Uso de CPU em repouso: **0,0%**.

---

## 5. Testes e Verificação Automatizada

Uma suíte de testes de integração ponta a ponta está incluída para validar o proxy contra uma instância em execução do ProPresenter 7:

```bash
cargo test --test integration_test -- --nocapture
```

A suíte valida:
1. **Fronteira de Autenticação**: Acessos não autenticados a `/` e `/html/pages/messages` redirecionam (`302`) para `/login`, e chamadas à API retornam `401 Unauthorized`.
2. **Whitelist de Segurança**: Rotas bloqueadas (`/v1/control`, `/v1/library`, `/v1/looks`) retornam `403 Forbidden`.
3. **Resiliência do Standby**: A tela de espera é exibida corretamente quando a porta do ProPresenter está inacessível.
4. **Envio Dinâmico de Mensagens**: Descobre dinamicamente a primeira mensagem configurada no ProPresenter (priorizando mensagens com "Permitir Notificações Web" ativado), inspeciona todos os argumentos/tokens definidos, preenche cada um com a palavra `"Test"` independentemente do nome ou quantidade de parâmetros, envia a requisição pelo proxy autenticado, valida o código `204 No Content` e limpa a mensagem da tela em seguida.

---

## 6. Configuração de Mensagens no ProPresenter 7

Para que as mensagens fiquem acessíveis na interface web (`/html/pages/messages`) e na suíte de testes de integração, elas devem ser expostas no ProPresenter 7:

1. **Abrir Mensagens**: No ProPresenter, acesse **Visualizar &rarr; Mensagens** (ou pressione `Control + M`, ou clique no ícone de Mensagens na barra superior).
2. **Criar ou Selecionar Mensagem**: Clique no botão **`+`** para criar uma nova mensagem (por exemplo, `Parent Pickup`).
3. **Definir o Modelo da Mensagem**: Digite o texto da mensagem usando tokens entre chaves, por exemplo:
   ```text
   {Guardian}, please come to the Nursery for {Child}.
   ```
4. **Definir Tokens**: Clique no botão de tokens no editor e adicione tokens de texto correspondentes aos nomes usados no modelo:
   - Token 1: `Guardian`
   - Token 2: `Child`
5. **Associar um Tema**: Escolha um tema ativo que possua elemento de texto (ex.: `Alerts`).
6. **Habilitar Exposição Web (Fundamental)**: Marque a caixa de seleção **"Allow Web Notifications"** (ou "Permitir Notificações Web").
   > *O ProPresenter só expõe mensagens para `/html/pages/messages` e `/v1/messages` se essa opção estiver marcada.*
7. **Verificação de Rede**: Em **Ajustes &rarr; Rede** do ProPresenter, verifique se a opção **"Ativar Rede"** está habilitada na porta `1025`.

> [!NOTE]
> **Testes Dinâmicos e Independentes da Máquina**: O teste de integração não depende de nenhum nome específico de mensagem ou esquema fixo de tokens. Ele consulta o ProPresenter em tempo de execução, lê a primeira mensagem configurada, extrai os tokens existentes e preenche todos com `"Test"`.
