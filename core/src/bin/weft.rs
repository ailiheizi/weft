use anyhow::{anyhow, bail, Context, Result};
use weft_core::service_manager::{
    platform_service_manager, resolve_install_options, PlatformServiceManager,
};

const DEFAULT_CORE_URL: &str = "http://127.0.0.1:17830";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Current {
        app: String,
        core_url: String,
    },
    List {
        app: String,
        core_url: String,
    },
    SceneList {
        app: String,
        core_url: String,
    },
    SceneShow {
        app: String,
        scene: String,
        core_url: String,
    },
    SceneCreate {
        app: String,
        scene: String,
        core_url: String,
    },
    SceneBind {
        app: String,
        scene: String,
        core_url: String,
    },
    Diff {
        app: String,
        from: String,
        to: String,
        core_url: String,
    },
    Generate {
        app: String,
        core_url: String,
    },
    Verify {
        app: String,
        core_url: String,
    },
    Activate {
        app: String,
        id: Option<String>,
        core_url: String,
    },
    Rollback {
        app: String,
        core_url: String,
    },
    InstallService {
        /// Raw args after "install-service" for resolve_install_options()
        raw: Vec<String>,
    },
    UninstallService {
        service_name: String,
    },
    StartService {
        service_name: String,
    },
    StopService {
        service_name: String,
    },
    ServiceStatus {
        service_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if is_help_args(&args) {
        println!("{}", usage());
        return Ok(());
    }

    let command = parse_args(&args)?;
    run(command).await
}

/// Builds an HTTP client that auto-injects the loopback bearer token (if found)
/// so `weft` works against a core with D2 auth enabled. Token resolution order:
/// 1) `WEFT_TOKEN` env var
/// 2) `<cwd>/data/runtime-token`
/// 3) `<cwd>/runtime-token`
/// Missing token → plain client (works against an unauthenticated core).
fn build_client() -> reqwest::Client {
    let token = std::env::var("WEFT_TOKEN").ok().filter(|t| !t.trim().is_empty()).or_else(|| {
        for candidate in ["data/runtime-token", "runtime-token"] {
            if let Ok(contents) = std::fs::read_to_string(candidate) {
                let trimmed = contents.trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
        }
        None
    });

    match token {
        Some(token) => {
            let mut headers = reqwest::header::HeaderMap::new();
            if let Ok(mut value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            {
                value.set_sensitive(true);
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
            reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        }
        None => reqwest::Client::new(),
    }
}

async fn run(command: Command) -> Result<()> {
    let client = build_client();

    match command {
        Command::Current { app, core_url } => {
            let value = fetch_json(
                &client,
                &format!("{}/api/apps/{}/generations", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value["active"])
        }
        Command::List { app, core_url } => {
            let value = fetch_json(
                &client,
                &format!("{}/api/apps/{}/generations", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value)
        }
        Command::SceneList { app, core_url } => {
            let value = fetch_json(
                &client,
                &format!("{}/api/apps/{}/scenes", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value)
        }
        Command::SceneShow {
            app,
            scene,
            core_url,
        } => {
            let value = fetch_json(
                &client,
                &format!("{}/api/apps/{}/scenes/{}", trim_url(&core_url), app, scene),
            )
            .await?;
            print_json(&value)
        }
        Command::SceneCreate {
            app,
            scene,
            core_url,
        } => {
            let value = post_json_body(
                &client,
                &format!("{}/api/apps/{}/scenes", trim_url(&core_url), app),
                serde_json::json!({ "name": scene }),
            )
            .await?;
            print_json(&value)
        }
        Command::SceneBind {
            app,
            scene,
            core_url,
        } => {
            let value = post_json_body(
                &client,
                &format!(
                    "{}/api/apps/{}/scenes/{}/bind",
                    trim_url(&core_url),
                    app,
                    scene
                ),
                serde_json::json!({}),
            )
            .await?;
            print_json(&value)
        }
        Command::Diff {
            app,
            from,
            to,
            core_url,
        } => {
            let value = fetch_json(
                &client,
                &format!(
                    "{}/api/apps/{}/generations/{}/diff/{}",
                    trim_url(&core_url),
                    app,
                    from,
                    to
                ),
            )
            .await?;
            print_json(&value)
        }
        Command::Generate { app, core_url } => {
            let value = post_json(
                &client,
                &format!("{}/api/apps/{}/propose", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value)
        }
        Command::Verify { app, core_url } => {
            let value = post_json(
                &client,
                &format!("{}/api/apps/{}/verify", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value)
        }
        Command::Activate { app, id, core_url } => {
            let url = match id {
                Some(id) => format!(
                    "{}/api/apps/{}/generations/{}/activate",
                    trim_url(&core_url),
                    app,
                    id
                ),
                None => format!("{}/api/apps/{}/activate", trim_url(&core_url), app),
            };
            let value = post_json(&client, &url).await?;
            print_json(&value)
        }
        Command::Rollback { app, core_url } => {
            let value = post_json(
                &client,
                &format!("{}/api/apps/{}/rollback", trim_url(&core_url), app),
            )
            .await?;
            print_json(&value)
        }
        Command::InstallService { raw } => {
            let opts = resolve_install_options(&raw)?;
            println!(
                "Installing service '{}' with binary: {}",
                opts.service_name,
                opts.binary_path.display()
            );
            println!("  config-dir: {}", opts.config_dir.display());
            println!("  data-dir:   {}", opts.data_dir.display());
            platform_service_manager().install(&opts)?;
            Ok(())
        }
        Command::UninstallService { service_name } => {
            platform_service_manager().uninstall(&service_name)?;
            Ok(())
        }
        Command::StartService { service_name } => {
            platform_service_manager().start(&service_name)?;
            Ok(())
        }
        Command::StopService { service_name } => {
            platform_service_manager().stop(&service_name)?;
            Ok(())
        }
        Command::ServiceStatus { service_name } => {
            let status = platform_service_manager().status(&service_name)?;
            let json = serde_json::to_string_pretty(&status)
                .context("failed to serialize service status")?;
            println!("{}", json);
            Ok(())
        }
    }
}

fn print_json(value: &serde_json::Value) -> Result<()> {
    let content = serde_json::to_string_pretty(value).context("response did not serialize")?;
    println!("{}", content);
    Ok(())
}

async fn fetch_json(client: &reqwest::Client, url: &str) -> Result<serde_json::Value> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request to {url} failed"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("failed reading response body from {url}"))?;

    if !status.is_success() {
        let detail = body.trim();
        if detail.is_empty() {
            bail!("request to {url} failed with status {status}");
        }

        bail!("request to {url} failed with status {status}: {detail}");
    }

    serde_json::from_str(&body).with_context(|| format!("response from {url} was not valid JSON"))
}

async fn post_json(client: &reqwest::Client, url: &str) -> Result<serde_json::Value> {
    let response = client
        .post(url)
        .json(&serde_json::json!({}))
        .send()
        .await
        .with_context(|| format!("request to {url} failed"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("failed reading response body from {url}"))?;

    if !status.is_success() {
        let detail = body.trim();
        if detail.is_empty() {
            bail!("request to {url} failed with status {status}");
        }

        bail!("request to {url} failed with status {status}: {detail}");
    }

    serde_json::from_str(&body).with_context(|| format!("response from {url} was not valid JSON"))
}

async fn post_json_body(
    client: &reqwest::Client,
    url: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value> {
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("request to {url} failed"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("failed reading response body from {url}"))?;

    if !status.is_success() {
        let detail = body.trim();
        if detail.is_empty() {
            bail!("request to {url} failed with status {status}");
        }

        bail!("request to {url} failed with status {status}: {detail}");
    }

    serde_json::from_str(&body).with_context(|| format!("response from {url} was not valid JSON"))
}

fn parse_service_name(args: &[String]) -> String {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--service-name" {
            if let Some(name) = args.get(i + 1) {
                return name.clone();
            }
        }
        i += 1;
    }
    "weft-core".to_string()
}

fn parse_args(args: &[String]) -> Result<Command> {
    match args.first().map(|value| value.as_str()) {
        Some("gen") => parse_gen_args(&args[1..]),
        Some("scene") => parse_scene_args(&args[1..]),
        Some("install-service") => Ok(Command::InstallService {
            raw: args[1..].to_vec(),
        }),
        Some("uninstall-service") => Ok(Command::UninstallService {
            service_name: parse_service_name(&args[1..]),
        }),
        Some("start-service") => Ok(Command::StartService {
            service_name: parse_service_name(&args[1..]),
        }),
        Some("stop-service") => Ok(Command::StopService {
            service_name: parse_service_name(&args[1..]),
        }),
        Some("service-status") => Ok(Command::ServiceStatus {
            service_name: parse_service_name(&args[1..]),
        }),
        _ => bail!(usage()),
    }
}

fn is_help_args(args: &[String]) -> bool {
    matches!(
        args.first().map(|value| value.as_str()),
        Some("--help" | "-h" | "help")
    )
}

fn parse_gen_args(args: &[String]) -> Result<Command> {
    match args.first().map(|value| value.as_str()) {
        Some("current") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::Current {
                app: positionals[0].clone(),
                core_url,
            })
        }
        Some("list") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::List {
                app: positionals[0].clone(),
                core_url,
            })
        }
        Some("diff") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 3 {
                bail!(usage());
            }

            Ok(Command::Diff {
                app: positionals[0].clone(),
                from: positionals[1].clone(),
                to: positionals[2].clone(),
                core_url,
            })
        }
        Some("generate") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::Generate {
                app: positionals[0].clone(),
                core_url,
            })
        }
        Some("verify") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::Verify {
                app: positionals[0].clone(),
                core_url,
            })
        }
        Some("activate") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if !(1..=2).contains(&positionals.len()) {
                bail!(usage());
            }

            Ok(Command::Activate {
                app: positionals[0].clone(),
                id: positionals.get(1).cloned(),
                core_url,
            })
        }
        Some("rollback") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::Rollback {
                app: positionals[0].clone(),
                core_url,
            })
        }
        _ => bail!(usage()),
    }
}

fn parse_scene_args(args: &[String]) -> Result<Command> {
    match args.first().map(|value| value.as_str()) {
        Some("list") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 1 {
                bail!(usage());
            }

            Ok(Command::SceneList {
                app: positionals[0].clone(),
                core_url,
            })
        }
        Some("show") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 2 {
                bail!(usage());
            }

            Ok(Command::SceneShow {
                app: positionals[0].clone(),
                scene: positionals[1].clone(),
                core_url,
            })
        }
        Some("create") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 2 {
                bail!(usage());
            }

            Ok(Command::SceneCreate {
                app: positionals[0].clone(),
                scene: positionals[1].clone(),
                core_url,
            })
        }
        Some("bind") => {
            let (positionals, core_url) = split_core_url_args(&args[1..])?;
            if positionals.len() != 2 {
                bail!(usage());
            }

            Ok(Command::SceneBind {
                app: positionals[0].clone(),
                scene: positionals[1].clone(),
                core_url,
            })
        }
        _ => bail!(usage()),
    }
}

fn split_core_url_args(args: &[String]) -> Result<(Vec<String>, String)> {
    let mut positionals = Vec::new();
    let mut core_url = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--core-url" => {
                if core_url.is_some() {
                    return Err(anyhow!(
                        "--core-url may only be provided once\n\n{}",
                        usage()
                    ));
                }

                let value = args
                    .get(index + 1)
                    .cloned()
                    .ok_or_else(|| anyhow!("--core-url requires a value\n\n{}", usage()))?;
                core_url = Some(value);
                index += 2;
            }
            other if other.starts_with("--") => {
                return Err(anyhow!("unknown flag: {other}\n\n{}", usage()));
            }
            _ => {
                positionals.push(args[index].clone());
                index += 1;
            }
        }
    }

    Ok((
        positionals,
        core_url.unwrap_or_else(|| DEFAULT_CORE_URL.to_string()),
    ))
}

fn trim_url(url: &str) -> &str {
    url.trim_end_matches('/')
}

fn usage() -> &'static str {
    "usage:
  weft gen current <app> [--core-url URL]
  weft gen list <app> [--core-url URL]
  weft gen diff <app> <from> <to> [--core-url URL]
  weft gen generate <app> [--core-url URL]
  weft gen verify <app> [--core-url URL]
  weft gen activate <app> [<id>] [--core-url URL]
  weft gen rollback <app> [--core-url URL]
  weft scene create <app> <scene> [--core-url URL]
  weft scene bind <app> <scene> [--core-url URL]
  weft scene list <app> [--core-url URL]
  weft scene show <app> <scene> [--core-url URL]

service management (requires elevated privileges):
  weft install-service [--path <DIR>] [--config <DIR>] [--data <DIR>] [--mode=system] [--service-name <NAME>]
  weft uninstall-service [--service-name <NAME>]
  weft start-service [--service-name <NAME>]
  weft stop-service [--service-name <NAME>]
  weft service-status [--service-name <NAME>]

  --path <DIR>         copy binary to DIR before installing (default: in-place)
  --config <DIR>       config directory (default: <binary-dir>/config)
  --data <DIR>         data directory (default: <binary-dir>/data)
  --mode=system        preset: use platform system paths
                         Windows: %ProgramData%\\WEFT\\
                         Linux:   /usr/local/bin, /etc/weft, /var/lib/weft
                         macOS:   /usr/local/bin, /Library/Application Support/WEFT"
}

#[cfg(test)]
mod tests {
    use super::{is_help_args, parse_args, run, trim_url, Command, DEFAULT_CORE_URL};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    async fn spawn_json_server(response: &'static str) -> (String, oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock server binds");
        let address = listener.local_addr().expect("mock server has address");
        let (request_tx, request_rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("mock server accepts request");
            let mut buffer = [0_u8; 4096];
            let bytes_read = stream
                .read(&mut buffer)
                .await
                .expect("mock server reads request");
            let request = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
            let _ = request_tx.send(request);
            let content_length = response.len();
            let http_response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {content_length}\r\nconnection: close\r\n\r\n{response}"
            );
            stream
                .write_all(http_response.as_bytes())
                .await
                .expect("mock server writes response");
        });

        (format!("http://{address}"), request_rx)
    }

    #[test]
    fn recognizes_help_args() {
        assert!(is_help_args(&strings(&["--help"])));
        assert!(is_help_args(&strings(&["-h"])));
        assert!(is_help_args(&strings(&["help"])));
        assert!(!is_help_args(&strings(&["gen", "list", "weft-code"])));
    }

    #[test]
    fn parses_current_with_default_core_url() {
        let command = parse_args(&strings(&["gen", "current", "weft-code"]))
            .expect("current command parses");

        assert_eq!(
            command,
            Command::Current {
                app: "weft-code".into(),
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn parses_list_with_explicit_core_url() {
        let command = parse_args(&strings(&[
            "gen",
            "list",
            "weft-code",
            "--core-url",
            "http://127.0.0.1:9999/",
        ]))
        .expect("list command parses");

        assert_eq!(
            command,
            Command::List {
                app: "weft-code".into(),
                core_url: "http://127.0.0.1:9999/".into(),
            }
        );
    }

    #[test]
    fn parses_diff_with_explicit_core_url() {
        let command = parse_args(&strings(&[
            "gen",
            "diff",
            "weft-code",
            "18",
            "19",
            "--core-url",
            "http://core.example",
        ]))
        .expect("diff command parses");

        assert_eq!(
            command,
            Command::Diff {
                app: "weft-code".into(),
                from: "18".into(),
                to: "19".into(),
                core_url: "http://core.example".into(),
            }
        );
    }

    #[test]
    fn parses_generate_with_default_core_url() {
        let command = parse_args(&strings(&["gen", "generate", "weft-code"]))
            .expect("generate command parses");

        assert_eq!(
            command,
            Command::Generate {
                app: "weft-code".into(),
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn parses_verify_with_explicit_core_url() {
        let command = parse_args(&strings(&[
            "gen",
            "verify",
            "weft-code",
            "--core-url",
            "http://core.example",
        ]))
        .expect("verify command parses");

        assert_eq!(
            command,
            Command::Verify {
                app: "weft-code".into(),
                core_url: "http://core.example".into(),
            }
        );
    }

    #[test]
    fn parses_activate_without_id() {
        let command = parse_args(&strings(&["gen", "activate", "weft-code"]))
            .expect("activate command without id parses");

        assert_eq!(
            command,
            Command::Activate {
                app: "weft-code".into(),
                id: None,
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn parses_activate_with_id_and_explicit_core_url() {
        let command = parse_args(&strings(&[
            "gen",
            "activate",
            "weft-code",
            "20",
            "--core-url",
            "http://core.example/",
        ]))
        .expect("activate command with id parses");

        assert_eq!(
            command,
            Command::Activate {
                app: "weft-code".into(),
                id: Some("20".into()),
                core_url: "http://core.example/".into(),
            }
        );
    }

    #[test]
    fn parses_rollback_with_default_core_url() {
        let command = parse_args(&strings(&["gen", "rollback", "weft-code"]))
            .expect("rollback command parses");

        assert_eq!(
            command,
            Command::Rollback {
                app: "weft-code".into(),
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn parses_scene_list_with_default_core_url() {
        let command = parse_args(&strings(&["scene", "list", "weft-code"]))
            .expect("scene list command parses");

        assert_eq!(
            command,
            Command::SceneList {
                app: "weft-code".into(),
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn parses_scene_show_with_explicit_core_url() {
        let command = parse_args(&strings(&[
            "scene",
            "show",
            "weft-code",
            "team",
            "--core-url",
            "http://core.example/",
        ]))
        .expect("scene show command parses");

        assert_eq!(
            command,
            Command::SceneShow {
                app: "weft-code".into(),
                scene: "team".into(),
                core_url: "http://core.example/".into(),
            }
        );
    }

    #[test]
    fn parses_scene_create_with_explicit_core_url() {
        let command = parse_args(&strings(&[
            "scene",
            "create",
            "weft-code",
            "team",
            "--core-url",
            "http://core.example",
        ]))
        .expect("scene create command parses");

        assert_eq!(
            command,
            Command::SceneCreate {
                app: "weft-code".into(),
                scene: "team".into(),
                core_url: "http://core.example".into(),
            }
        );
    }

    #[test]
    fn parses_scene_bind_with_default_core_url() {
        let command = parse_args(&strings(&["scene", "bind", "weft-code", "team"]))
            .expect("scene bind command parses");

        assert_eq!(
            command,
            Command::SceneBind {
                app: "weft-code".into(),
                scene: "team".into(),
                core_url: DEFAULT_CORE_URL.into(),
            }
        );
    }

    #[test]
    fn rejects_unknown_flag() {
        let error = parse_args(&strings(&["gen", "current", "weft-code", "--wat"]))
            .expect_err("unknown flag should fail");

        assert!(error.to_string().contains("unknown flag: --wat"));
    }

    #[test]
    fn rejects_missing_diff_argument() {
        let error = parse_args(&strings(&["gen", "diff", "weft-code", "18"]))
            .expect_err("missing diff arg should fail");

        assert!(error
            .to_string()
            .contains("weft gen diff <app> <from> <to>"));
    }

    #[test]
    fn rejects_scene_show_missing_scene_argument() {
        let error = parse_args(&strings(&["scene", "show", "weft-code"]))
            .expect_err("missing scene arg should fail");

        assert!(error
            .to_string()
            .contains("weft scene show <app> <scene> [--core-url URL]"));
        assert!(error
            .to_string()
            .contains("weft scene create <app> <scene> [--core-url URL]"));
        assert!(error
            .to_string()
            .contains("weft scene bind <app> <scene> [--core-url URL]"));
    }

    #[test]
    fn usage_includes_scene_commands() {
        let error = parse_args(&strings(&["wat"]))
            .expect_err("unknown top-level command should show usage");

        assert!(error
            .to_string()
            .contains("weft scene list <app> [--core-url URL]"));
        assert!(error
            .to_string()
            .contains("weft scene show <app> <scene> [--core-url URL]"));
    }

    #[test]
    fn trims_trailing_slash_from_core_url() {
        assert_eq!(trim_url("http://127.0.0.1:17830/"), "http://127.0.0.1:17830");
        assert_eq!(trim_url("http://127.0.0.1:17830"), "http://127.0.0.1:17830");
    }

    #[tokio::test]
    async fn scene_list_calls_core_endpoint_successfully() {
        let (core_url, request_rx) = spawn_json_server(r#"{"scenes":[]}"#).await;

        run(Command::SceneList {
            app: "weft-code".into(),
            core_url,
        })
        .await
        .expect("scene list command succeeds against mock core");

        let request = request_rx.await.expect("mock server captures request");
        assert!(request.starts_with("GET /api/apps/weft-code/scenes HTTP/1.1"));
    }

    #[tokio::test]
    async fn generate_calls_core_endpoint_successfully() {
        let (core_url, request_rx) = spawn_json_server(r#"{"generation":{"id":20}}"#).await;

        run(Command::Generate {
            app: "weft-code".into(),
            core_url,
        })
        .await
        .expect("generate command succeeds against mock core");

        let request = request_rx.await.expect("mock server captures request");
        assert!(request.starts_with("POST /api/apps/weft-code/propose HTTP/1.1"));
    }

    #[tokio::test]
    async fn scene_create_calls_core_endpoint_successfully() {
        let (core_url, request_rx) = spawn_json_server(r#"{"scene":{"name":"team"}}"#).await;

        run(Command::SceneCreate {
            app: "weft-code".into(),
            scene: "team".into(),
            core_url,
        })
        .await
        .expect("scene create command succeeds against mock core");

        let request = request_rx.await.expect("mock server captures request");
        assert!(request.starts_with("POST /api/apps/weft-code/scenes HTTP/1.1"));
        assert!(request.contains(r#"{"name":"team"}"#));
    }

    #[tokio::test]
    async fn scene_bind_calls_core_endpoint_successfully() {
        let (core_url, request_rx) = spawn_json_server(r#"{"active_scene":"team"}"#).await;

        run(Command::SceneBind {
            app: "weft-code".into(),
            scene: "team".into(),
            core_url,
        })
        .await
        .expect("scene bind command succeeds against mock core");

        let request = request_rx.await.expect("mock server captures request");
        assert!(request.starts_with("POST /api/apps/weft-code/scenes/team/bind HTTP/1.1"));
    }
}
