#!/bin/sh
set -eu

REPO="avynnx/mcping"
BIN="mcping"

say() {
    printf '%s\n' "$*"
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        die "curl or wget is required"
    fi
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$arch" in
        x86_64 | amd64) arch="x86_64" ;;
        aarch64 | arm64) arch="aarch64" ;;
        *) die "unsupported CPU: $arch" ;;
    esac

    case "$os" in
        Linux) echo "$arch-unknown-linux-musl" ;;
        Darwin) echo "$arch-apple-darwin" ;;
        *) die "unsupported OS: $os (on Windows, run: irm avynnx.dev/mcping/install.ps1 | iex)" ;;
    esac
}

verify() {
    expected=$(cut -d ' ' -f 1 "$2")
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$1" | cut -d ' ' -f 1)
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$1" | cut -d ' ' -f 1)
    else
        say "warning: no sha256sum or shasum found, skipping checksum"
        return
    fi
    [ "$actual" = "$expected" ] || die "checksum mismatch, download may be corrupted"
}

main() {
    target=$(detect_target)
    version="${MCPING_VERSION:-latest}"
    dir="${MCPING_INSTALL_DIR:-$HOME/.local/bin}"

    if [ "$version" = "latest" ]; then
        base="https://github.com/$REPO/releases/latest/download"
    else
        base="https://github.com/$REPO/releases/download/$version"
    fi

    archive="$BIN-$target.tar.gz"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    say "downloading $BIN ($target)..."
    fetch "$base/$archive" "$tmp/$archive" || die "download failed: $base/$archive"
    fetch "$base/$archive.sha256" "$tmp/$archive.sha256" || die "checksum download failed"
    verify "$tmp/$archive" "$tmp/$archive.sha256"

    tar -xzf "$tmp/$archive" -C "$tmp"
    mkdir -p "$dir"
    mv "$tmp/$BIN" "$dir/$BIN"
    chmod +x "$dir/$BIN"

    say "installed $BIN to $dir/$BIN"

    case ":$PATH:" in
        *":$dir:"*)
            say "run: $BIN mc.hypixel.net"
            ;;
        *)
            case "${SHELL:-}" in
                */zsh) rc="$HOME/.zshrc" ;;
                */bash) rc="$HOME/.bashrc" ;;
                */fish) rc="$HOME/.config/fish/config.fish" ;;
                *) rc="$HOME/.profile" ;;
            esac
            say ""
            say "$dir is not on your PATH. add it with:"
            if [ "${rc##*/}" = "config.fish" ]; then
                say "  fish_add_path $dir"
            else
                say "  echo 'export PATH=\"$dir:\$PATH\"' >> $rc && . $rc"
            fi
            say "then run: $BIN mc.hypixel.net"
            ;;
    esac
}

main "$@"
