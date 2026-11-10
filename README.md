# mcping

Ping Minecraft Java servers from your terminal.

![mcping pinging Hypixel and DonutSMP](assets/demo.svg)

- Server icons and colored MOTDs, right in the terminal
- Pings any number of servers at once
- Follows SRV records like the game does
- Real latency from the ping packet, not just the TCP connect
- JSON output for scripts

## Install

| Platform | Command |
|---|---|
| Linux / macOS | `curl -fsSL avynnx.dev/mcping/install.sh \| sh` |
| Windows | `irm avynnx.dev/mcping/install.ps1 \| iex` |
| Nix | `nix run github:avynnx/mcping` |
| Cargo | `cargo install --git https://github.com/avynnx/mcping` |

## Usage

```
mcping mc.hypixel.net
mcping 2b2t.org donutsmp.net
mcping 10.0.0.5:25570
mcping -v play.example.net
mcping -j mc.hypixel.net | jq '.[].latency_ms'
```

```
-t, --timeout <SECS>   per-server timeout [default: 5]
-j, --json             print JSON instead
-n, --no-icon          don't draw the server icon
-s, --icon-size <PX>   icon size in pixels, 2-64 [default: 16]
-v, --verbose          show each step of the ping
-u, --uninstall        uninstall mcping
-h, --help             print help
-V, --version          print version
```

`-v` shows the DNS lookup, connection and every packet, which helps when a server won't answer.

## Uninstall

`mcping -u` removes it, or tells you the right command if you installed it with winget, Cargo or Nix.

## License

MIT
