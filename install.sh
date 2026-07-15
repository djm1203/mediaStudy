#!/bin/sh
# The Librarian — one-line installer (B-006).
#
#   curl -fsSL https://raw.githubusercontent.com/djm1203/mediaStudy/main/install.sh | sh
#
# Downloads the latest prebuilt `librarian` binary for your OS/arch from GitHub
# releases and installs it to ~/.local/bin (override with LIBRARIAN_INSTALL_DIR).
# Linux/macOS only; on Windows use the release .zip. Requires curl + tar.
set -eu

REPO="djm1203/mediaStudy"
BIN="librarian"
INSTALL_DIR="${LIBRARIAN_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux) plat="linux" ;;
    Darwin) plat="macos" ;;
    *)
        echo "Unsupported OS: $os. On Windows, download the .zip from:" >&2
        echo "  https://github.com/${REPO}/releases/latest" >&2
        exit 1
        ;;
esac

case "$arch" in
    x86_64 | amd64) a="x86_64" ;;
    arm64 | aarch64) a="aarch64" ;;
    *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
esac

# Only macOS ships an aarch64 build today; Linux is x86_64-only for now.
if [ "$plat" = "linux" ] && [ "$a" != "x86_64" ]; then
    echo "No prebuilt Linux $a binary yet — build from source: cargo install --path ." >&2
    exit 1
fi

asset="${BIN}-${plat}-${a}.tar.gz"

echo "Resolving latest release..."
tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 \
    | sed -E 's/.*"tag_name" *: *"([^"]+)".*/\1/')"
[ -n "$tag" ] || { echo "Could not determine the latest release." >&2; exit 1; }

url="https://github.com/${REPO}/releases/download/${tag}/${asset}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading ${BIN} ${tag} (${asset})..."
curl -fsSL "$url" -o "$tmp/$asset"
tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$INSTALL_DIR"
if command -v install >/dev/null 2>&1; then
    install -m 0755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"
else
    cp "$tmp/$BIN" "$INSTALL_DIR/$BIN"
    chmod 0755 "$INSTALL_DIR/$BIN"
fi

echo "Installed ${BIN} to ${INSTALL_DIR}/${BIN}"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "Note: add it to your PATH — export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

"$INSTALL_DIR/$BIN" --version 2>/dev/null || true
