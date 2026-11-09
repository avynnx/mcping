# mcping

Ping Minecraft Java servers from your terminal.

![mcping pinging Hypixel and DonutSMP](assets/demo.svg)

- The server icon drawn next to the output with truecolor half blocks
- MOTDs in their real colors (legacy `§` codes, chat components and hex colors)
- Pings any number of servers at once
- Follows `_minecraft._tcp` SRV records like the game does
- Real latency from the ping/pong packets, not just the TCP connect time
- `--json` output for scripts, and a non-zero exit code when a server is down

The protocol is written by hand (VarInts, handshake, status and ping packets), so the only network dependencies are Tokio and a DNS resolver.

## Install

Linux and macOS:

```
curl -fsSL avynnx.dev/mcping/install.sh | sh
```

Windows:

```
winget install avynnx.mcping
```

Nix:

```
nix run github:avynnx/mcping -- mc.hypixel.net
```

With Cargo:

```
cargo install --git https://github.com/avynnx/mcping
```

The install script puts `mcping` in `~/.local/bin`. Set `MCPING_INSTALL_DIR` to change that, or `MCPING_VERSION=v0.1.0` to pin a version.

## Usage

```
mcping <SERVER>...

  -t, --timeout <SECS>   per-server timeout [default: 5]
  -j, --json             print JSON instead
  -n, --no-icon          don't draw the server icon
  -s, --icon-size <PX>   icon size in pixels, 2-64 [default: 16]
  -v, --verbose          show each step of the ping
  -u, --uninstall        uninstall mcping
  -h, --help             print help
  -V, --version          print version
```

Servers can be `host`, `host:port`, an IP or `[ipv6]:port`. Colors and icons turn off when output isn't a terminal or `NO_COLOR` is set, and `CLICOLOR_FORCE=1` keeps them on.

```
mcping play.example.net 10.0.0.5:25570 --json | jq '.[] | {server, latency_ms}'
```

`-v` shows every step, which helps when a server won't answer:

```
$ mcping -v 2b2t.org
→ SRV _minecraft._tcp.2b2t.org -> connect.2b2t.org:25565 (23 ms)
→ resolved connect.2b2t.org -> 40.223.14.133 (54 ms)
→ connecting to 40.223.14.133:25565
→ connected to 40.223.14.133:25565 (10 ms)
→ sent handshake and status request (21 bytes)
→ got status response (12814 bytes, 56 ms)
→ sent ping (10 bytes)
→ got pong (11 ms)
● 2b2t.org → connect.2b2t.org:25565  11 ms
  ...
```

With `--json`, the steps go to stderr so the JSON stays clean.

## Uninstall

```
mcping -u
```

If you used the install script, this removes mcping. If you installed it with winget, Cargo or Nix, it tells you the command to run instead:

| Installed with | Uninstall with |
|---|---|
| install script | `mcping -u` or `rm ~/.local/bin/mcping` |
| winget | `winget uninstall avynnx.mcping` |
| Cargo | `cargo uninstall mcping` |
| Nix | `nix profile remove mcping` |

## License

MIT
