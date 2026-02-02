#!/usr/bin/env bash
# OpenSAM installer (one-liner friendly)

set -euo pipefail

BANNER="◆ OpenSAM Installer"
REPO_TARBALL="https://github.com/prfagit/opensam/archive/refs/heads/main.tar.gz"
RUSTUP_URL="https://sh.rustup.rs"

say() { printf "%s\n" "$*"; }
fail() { say "ERROR: $*" >&2; exit 1; }
need_cmd() { command -v "$1" >/dev/null 2>&1 || fail "Missing dependency: $1"; }

say "$BANNER"

OS="$(uname -s)"
ARCH="$(uname -m)"

say "Detected: $OS $ARCH"

need_cmd curl
need_cmd tar

if ! command -v cargo >/dev/null 2>&1; then
  say "Rust not found. Installing Rust (rustup)..."
  curl -sSf "$RUSTUP_URL" | sh -s -- -y
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

say "Downloading OpenSAM source..."

curl -sL "$REPO_TARBALL" | tar -xz -C "$TMP_DIR"

SRC_DIR="$TMP_DIR/opensam-main"
[ -d "$SRC_DIR" ] || fail "Failed to unpack source"

say "Building (release)..."
cd "$SRC_DIR"

cargo build --release

if [ "$OS" = "Darwin" ]; then
  INSTALL_DIR="$HOME/.cargo/bin"
else
  INSTALL_DIR="$HOME/.local/bin"
fi

mkdir -p "$INSTALL_DIR"

cp "target/release/opensam" "$INSTALL_DIR/opensam"
ln -sf "$INSTALL_DIR/opensam" "$INSTALL_DIR/sam"

say "Installed to: $INSTALL_DIR/opensam"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  say ""
  say "Add this to your shell profile to use 'opensam' globally:"
  say "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

say ""
say "Starting setup wizard..."
"$INSTALL_DIR/opensam" setup
