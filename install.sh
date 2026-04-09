#!/usr/bin/env bash
# tinypng-cli installer.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aototo/tinypng-cli/main/install.sh | bash
#
# Environment variables:
#   TINYPNG_VERSION   Version tag to install (default: latest)
#   TINYPNG_INSTALL_DIR   Destination directory (default: $HOME/.local/bin)

set -euo pipefail

REPO="aototo/tinypng-cli"
BINARY="tinypng"
INSTALL_DIR="${TINYPNG_INSTALL_DIR:-$HOME/.local/bin}"

say()  { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

# Detect OS + arch -> Rust target triple.
detect_target() {
  local os arch
  case "$(uname -s)" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    *) die "unsupported OS: $(uname -s). Use the npm package instead: npm i -g @aototo/tinypng-cli" ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) die "unsupported arch: $(uname -m)" ;;
  esac
  printf '%s-%s' "$arch" "$os"
}

# Resolve version: use TINYPNG_VERSION or ask the GitHub API for the latest tag.
resolve_version() {
  if [ -n "${TINYPNG_VERSION:-}" ]; then
    printf '%s' "${TINYPNG_VERSION#v}"
    return
  fi
  local api="https://api.github.com/repos/${REPO}/releases/latest"
  local tag
  if command -v curl >/dev/null 2>&1; then
    tag=$(curl -fsSL "$api" | grep -o '"tag_name":[[:space:]]*"[^"]*"' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')
  elif command -v wget >/dev/null 2>&1; then
    tag=$(wget -qO- "$api" | grep -o '"tag_name":[[:space:]]*"[^"]*"' | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')
  else
    die "need curl or wget to fetch the latest version"
  fi
  [ -n "$tag" ] || die "could not determine latest release; set TINYPNG_VERSION=X.Y.Z to override"
  printf '%s' "${tag#v}"
}

main() {
  command -v tar >/dev/null 2>&1 || die "need 'tar' to extract the archive"

  local target version archive url tmp
  target=$(detect_target)
  version=$(resolve_version)
  archive="tinypng-cli-v${version}-${target}.tar.gz"
  url="https://github.com/${REPO}/releases/download/v${version}/${archive}"

  say "installing tinypng-cli v${version} for ${target}"
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  say "downloading ${url}"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --progress-bar -o "${tmp}/${archive}" "$url"
  else
    wget -O "${tmp}/${archive}" "$url"
  fi

  say "extracting"
  tar -xzf "${tmp}/${archive}" -C "$tmp"

  [ -f "${tmp}/${BINARY}" ] || die "archive did not contain expected binary '${BINARY}'"

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "${tmp}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

  say "installed ${INSTALL_DIR}/${BINARY}"

  if ! command -v "${BINARY}" >/dev/null 2>&1; then
    warn "${INSTALL_DIR} is not on your PATH."
    warn "Add this line to your shell profile (~/.bashrc, ~/.zshrc):"
    warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  else
    "${BINARY}" --version || true
  fi

  say "next steps: get a free TinyPNG API key at https://tinypng.com/developers"
  say "then run: tinypng keys add <your-key>"
}

main "$@"
