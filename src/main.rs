mod icon;
mod protocol;
mod status;
mod uninstall;

use std::io::IsTerminal;
use std::net::IpAddr;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use hickory_resolver::TokioResolver;
use hickory_resolver::proto::rr::RData;
use owo_colors::{OwoColorize, Style};
use serde_json::{Value, json};

use status::Status;

const DEFAULT_PORT: u16 = 25565;

#[derive(Parser)]
#[command(version, about = "Ping Minecraft Java servers from your terminal")]
struct Args {
    #[arg(
        required_unless_present = "uninstall",
        value_name = "SERVER",
        help = "host, host:port, IP or [ipv6]:port"
    )]
    servers: Vec<String>,

    #[arg(
        short,
        long,
        default_value_t = 5,
        value_name = "SECS",
        help = "Per-server timeout"
    )]
    timeout: u64,

    #[arg(long, help = "Print JSON instead")]
    json: bool,

    #[arg(long, help = "Don't draw the server icon")]
    no_icon: bool,

    #[arg(
        long,
        default_value_t = 16,
        value_name = "PX",
        value_parser = clap::value_parser!(u16).range(2..=64),
        help = "Icon size in pixels"
    )]
    icon_size: u16,

    #[arg(short, long, help = "Uninstall mcping")]
    uninstall: bool,
}

struct Reply {
    target: String,
    status: Status,
    raw: Value,
    latency: Duration,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    if args.uninstall {
        return match uninstall::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::FAILURE
            }
        };
    }

    let timeout = Duration::from_secs(args.timeout);
    let forced = std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0");
    let color =
        (forced || std::io::stdout().is_terminal()) && std::env::var_os("NO_COLOR").is_none();
    let icon_size = (color && !args.no_icon).then_some(usize::from(args.icon_size));

    let tasks: Vec<_> = args
        .servers
        .into_iter()
        .map(|server| {
            tokio::spawn(async move {
                let result = tokio::time::timeout(timeout, query(&server))
                    .await
                    .unwrap_or_else(|_| Err(anyhow!("timed out")));
                (server, result)
            })
        })
        .collect();

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("{e}"),
        }
    }

    let failed = results.iter().any(|(_, result)| result.is_err());
    if args.json {
        print_json(&results);
    } else {
        for (server, result) in &results {
            match result {
                Ok(reply) => print_reply(server, reply, color, icon_size),
                Err(e) => print_error(server, e, color),
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn query(server: &str) -> Result<Reply> {
    let (host, port) = split_address(server)?;
    let (target, target_port) = match port {
        Some(port) => (host.to_string(), port),
        None => resolve_srv(host)
            .await
            .unwrap_or_else(|| (host.to_string(), DEFAULT_PORT)),
    };

    let pong = protocol::ping((&target, target_port), host, target_port).await?;
    let mut raw: Value = serde_json::from_str(&pong.json).context("server sent invalid JSON")?;
    if let Some(players) = raw.get_mut("players").and_then(Value::as_object_mut) {
        players.remove("sample");
    }
    let status = serde_json::from_value(raw.clone()).context("server sent an unexpected status")?;

    Ok(Reply {
        target: format!("{target}:{target_port}"),
        status,
        raw,
        latency: pong.latency,
    })
}

fn split_address(server: &str) -> Result<(&str, Option<u16>)> {
    if let Some(rest) = server.strip_prefix('[') {
        let (host, rest) = rest.split_once(']').context("missing ']' in address")?;
        let port = match rest.strip_prefix(':') {
            Some(port) => Some(port.parse().context("invalid port")?),
            None => None,
        };
        return Ok((host, port));
    }
    match server.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            Ok((host, Some(port.parse().context("invalid port")?)))
        }
        _ => Ok((server, None)),
    }
}

async fn resolve_srv(host: &str) -> Option<(String, u16)> {
    if host.parse::<IpAddr>().is_ok() || host == "localhost" {
        return None;
    }
    let resolver = TokioResolver::builder_tokio().ok()?.build().ok()?;
    let lookup = resolver
        .srv_lookup(format!("_minecraft._tcp.{host}."))
        .await
        .ok()?;
    lookup
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::SRV(srv) => Some(srv),
            _ => None,
        })
        .min_by_key(|srv| srv.priority)
        .map(|srv| {
            let target = srv.target.to_utf8();
            (target.trim_end_matches('.').to_string(), srv.port)
        })
}

fn styled(text: &str, style: &status::Style) -> String {
    let mut out = Style::new();
    if let Some((r, g, b)) = style.color {
        out = out.truecolor(r, g, b);
    }
    if style.bold {
        out = out.bold();
    }
    if style.italic {
        out = out.italic();
    }
    text.style(out).to_string()
}

fn motd_lines(spans: &[status::Span], color: bool) -> Vec<String> {
    let mut lines: Vec<Vec<(&str, &status::Style)>> = vec![Vec::new()];
    for span in spans {
        for (i, part) in span.text.split('\n').enumerate() {
            if i > 0 {
                lines.push(Vec::new());
            }
            if let Some(line) = lines.last_mut() {
                line.push((part, &span.style));
            }
        }
    }

    lines
        .into_iter()
        .filter_map(|mut parts| {
            while parts
                .first()
                .is_some_and(|(text, _)| text.trim().is_empty())
            {
                parts.remove(0);
            }
            while parts.last().is_some_and(|(text, _)| text.trim().is_empty()) {
                parts.pop();
            }
            let last = parts.len().checked_sub(1)?;
            let mut line = String::new();
            for (i, (text, style)) in parts.into_iter().enumerate() {
                let text = if i == 0 { text.trim_start() } else { text };
                let text = if i == last { text.trim_end() } else { text };
                if color {
                    line.push_str(&styled(text, style));
                } else {
                    line.push_str(text);
                }
            }
            Some(line)
        })
        .collect()
}

fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn print_reply(server: &str, reply: &Reply, color: bool, icon_size: Option<usize>) {
    let ms = reply.latency.as_millis();
    let latency = format!("{ms} ms");
    let target = if reply.target.starts_with(server) {
        String::new()
    } else {
        format!(" → {}", reply.target)
    };

    let mut lines = Vec::new();
    if color {
        let latency_style = match ms {
            0..80 => Style::new().green(),
            80..200 => Style::new().yellow(),
            _ => Style::new().red(),
        };
        lines.push(format!(
            "{} {}{}  {}",
            "●".green(),
            server.bold(),
            target.dimmed(),
            latency.style(latency_style)
        ));
    } else {
        lines.push(format!("● {server}{target}  {latency}"));
    }

    for line in motd_lines(&status::spans(&reply.status.description), color) {
        lines.push(format!("  {line}"));
    }

    let version = &reply.status.version;
    let version = format!(
        "{} (protocol {})",
        status::plain(&status::spans(&Value::String(version.name.clone()))).trim(),
        version.protocol
    );
    let info = match &reply.status.players {
        Some(players) => format!(
            "{}/{} players · {version}",
            thousands(players.online),
            thousands(players.max)
        ),
        None => version,
    };
    if color {
        lines.push(format!("  {}", info.dimmed()));
    } else {
        lines.push(format!("  {info}"));
    }

    let icon = icon_size.and_then(|size| {
        let image = icon::decode(reply.status.favicon.as_deref()?)?;
        Some((image.render(size), size))
    });
    match icon {
        Some((icon, width)) => {
            for i in 0..icon.len().max(lines.len()) {
                let left = icon.get(i).map_or_else(|| " ".repeat(width), Clone::clone);
                println!("{left}  {}", lines.get(i).map_or("", String::as_str));
            }
        }
        None => {
            for line in lines {
                println!("{line}");
            }
        }
    }
    println!();
}

fn print_error(server: &str, error: &anyhow::Error, color: bool) {
    let message = format!("{error:#}");
    if color {
        println!("{} {}  {}\n", "✗".red(), server.bold(), message.red());
    } else {
        println!("✗ {server}  {message}\n");
    }
}

fn print_json(results: &[(String, Result<Reply>)]) {
    let out: Vec<Value> = results
        .iter()
        .map(|(server, result)| match result {
            Ok(reply) => json!({
                "server": server,
                "online": true,
                "address": reply.target,
                "latency_ms": reply.latency.as_millis() as u64,
                "status": reply.raw,
            }),
            Err(e) => json!({
                "server": server,
                "online": false,
                "error": format!("{e:#}"),
            }),
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::{motd_lines, split_address, status, thousands};
    use serde_json::json;

    #[test]
    fn motd_drops_centering_whitespace() {
        let spans = status::spans(&json!("        §aHypixel §c[1.8]  \n   \n     §eSKYBLOCK"));
        assert_eq!(motd_lines(&spans, false), ["Hypixel [1.8]", "SKYBLOCK"]);
    }

    #[test]
    fn formats_thousands() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(25679), "25,679");
        assert_eq!(thousands(200000), "200,000");
        assert_eq!(thousands(-1234567), "-1,234,567");
    }

    #[test]
    fn splits_addresses() {
        assert_eq!(
            split_address("play.example.net").unwrap(),
            ("play.example.net", None)
        );
        assert_eq!(
            split_address("1.2.3.4:25570").unwrap(),
            ("1.2.3.4", Some(25570))
        );
        assert_eq!(split_address("[::1]:25565").unwrap(), ("::1", Some(25565)));
        assert_eq!(split_address("::1").unwrap(), ("::1", None));
        assert!(split_address("host:notaport").is_err());
    }
}
