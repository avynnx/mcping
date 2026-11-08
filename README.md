# mcping

Ping Minecraft Java servers from your terminal.

```
$ mcping mc.hypixel.net donutsmp.net
● mc.hypixel.net  32 ms
  Hypixel Network [1.8/26.3]
  SKYBLOCK GHOST VISITORS
  25,396/200,000 players · Requires MC 1.8 / 1.21 (protocol 47)

● donutsmp.net → java.donutsmp.net:25565  41 ms
  DonutSMP.net
  ѕᴜʀᴠɪᴠᴀʟ
  18,670/50,000 players · Velocity 1.7.2-26.3 (protocol 777)
```

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
      --json             print JSON instead
      --no-icon          don't draw the server icon
      --icon-size <PX>   icon size in pixels, 2-64 [default: 16]
```

Servers can be `host`, `host:port`, an IP or `[ipv6]:port`. Colors and icons turn off when output isn't a terminal or `NO_COLOR` is set.

```
mcping play.example.net 10.0.0.5:25570 --json | jq '.[] | {server, latency_ms}'
```

## License

MIT
