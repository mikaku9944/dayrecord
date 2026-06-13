#!/usr/bin/env bash
# Install DayRecord CLI from GitHub Release to ~/.local/bin for MCP.
# Usage:
#   ./scripts/install.sh
#   ./scripts/install.sh --version 0.1.1 --write-config
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="${REPO:-mikaku9944/dayrecord}"
VERSION=""
WRITE_CONFIG=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="${2#v}"; shift 2 ;;
    --write-config) WRITE_CONFIG=1; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  echo "Resolving latest release..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"v\?\([^"]*\)".*/\1/p' | head -1)"
fi

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS-$ARCH" in
  Darwin-arm64|Darwin-aarch64)
    ARTIFACT="dayrecord-${VERSION}-aarch64-apple-darwin.tar.gz"
    BIN="dayrecord"
    ;;
  Linux-x86_64|Linux-amd64)
    ARTIFACT="dayrecord-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
    BIN="dayrecord"
    ;;
  *)
    echo "Unsupported platform: $OS $ARCH"
    exit 1
    ;;
esac

INSTALL_DIR="${HOME}/.local/bin"
INSTALL_EXE="${INSTALL_DIR}/${BIN}"
mkdir -p "$INSTALL_DIR"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
BASE="https://github.com/${REPO}/releases/download/v${VERSION}"

echo "Installing DayRecord ${VERSION}..."
curl -fsSL "${BASE}/${ARTIFACT}" -o "${TMP}/${ARTIFACT}"
curl -fsSL "${BASE}/SHA256SUMS.txt" -o "${TMP}/SHA256SUMS.txt"

EXPECTED="$(grep " ${ARTIFACT}\$" "${TMP}/SHA256SUMS.txt" | awk '{print $1}')"
ACTUAL="$(sha256sum "${TMP}/${ARTIFACT}" | awk '{print $1}')"
if [[ "$EXPECTED" != "$ACTUAL" ]]; then
  echo "Checksum mismatch for ${ARTIFACT}"
  exit 1
fi
echo "Checksum ok."

tar xzf "${TMP}/${ARTIFACT}" -C "$TMP"
FOUND="$(find "$TMP" -name "$BIN" -type f | head -1)"
if [[ -z "$FOUND" ]]; then
  echo "Could not find $BIN in archive"
  exit 1
fi

cp "$FOUND" "$INSTALL_EXE"
chmod +x "$INSTALL_EXE"
echo "Installed: $INSTALL_EXE"

CURSOR_CONFIG="${HOME}/.cursor/mcp.json"
CODEX_CONFIG="${HOME}/.codex/config.toml"

cat <<EOF

=== Cursor mcp.json snippet ===
{
  "mcpServers": {
    "dayrecord": {
      "command": "${INSTALL_EXE}",
      "args": ["mcp"]
    }
  }
}

=== Codex config.toml snippet ===
[mcp_servers.dayrecord]
command = '${INSTALL_EXE}'
args = ["mcp"]
EOF

if [[ "$WRITE_CONFIG" -eq 1 ]]; then
  mkdir -p "$(dirname "$CURSOR_CONFIG")"
  if [[ -f "$CURSOR_CONFIG" ]]; then
    python3 - <<PY
import json, pathlib
p = pathlib.Path("$CURSOR_CONFIG")
data = json.loads(p.read_text()) if p.read_text().strip() else {}
data.setdefault("mcpServers", {})["dayrecord"] = {"command": "$INSTALL_EXE", "args": ["mcp"]}
p.write_text(json.dumps(data, indent=2) + "\n")
PY
    echo "Updated $CURSOR_CONFIG"
  else
    printf '%s\n' "{\"mcpServers\":{\"dayrecord\":{\"command\":\"$INSTALL_EXE\",\"args\":[\"mcp\"]}}}" > "$CURSOR_CONFIG"
    echo "Created $CURSOR_CONFIG"
  fi

  mkdir -p "$(dirname "$CODEX_CONFIG")"
  if [[ -f "$CODEX_CONFIG" ]] && grep -q '\[mcp_servers.dayrecord\]' "$CODEX_CONFIG"; then
    echo "Codex config already has [mcp_servers.dayrecord]"
  else
    {
      echo ""
      echo "[mcp_servers.dayrecord]"
      echo "command = '$INSTALL_EXE'"
      echo 'args = ["mcp"]'
    } >> "$CODEX_CONFIG"
    echo "Appended to $CODEX_CONFIG"
  fi
fi

echo ""
echo "Next: enable dayrecord in your IDE MCP settings."
echo "Running doctor mcp..."
"$INSTALL_EXE" doctor mcp
