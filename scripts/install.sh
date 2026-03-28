#!/usr/bin/env sh
set -eu

REPO="kerk99/itylos-cli"
BINARY="itylos"
VERSION="${ITYLOS_VERSION:-latest}"
INSTALL_DIR="${ITYLOS_INSTALL_DIR:-$HOME/.local/bin}"

detect_os() {
  os_name="$(uname -s)"
  arch_name="$(detect_arch)"
  case "$os_name" in
    Linux)
      if [ "$arch_name" = "aarch64" ]; then
        printf '%s' "unknown-linux-gnu"
      else
        printf '%s' "unknown-linux-musl"
      fi
      ;;
    Darwin) printf '%s' "apple-darwin" ;;
    *)
      echo "Unsupported OS: $os_name" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf '%s' "x86_64" ;;
    arm64|aarch64) printf '%s' "aarch64" ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

ensure_downloader() {
  if command -v curl >/dev/null 2>&1; then
    DOWNLOADER="curl -fsSL"
    return
  fi

  if command -v wget >/dev/null 2>&1; then
    DOWNLOADER="wget -qO-"
    return
  fi

  echo "curl or wget is required" >&2
  exit 1
}

release_api_url() {
  if [ "$VERSION" = "latest" ]; then
    printf '%s' "https://api.github.com/repos/$REPO/releases/latest"
  else
    printf '%s' "https://api.github.com/repos/$REPO/releases/tags/$VERSION"
  fi
}

asset_name() {
  arch="$(detect_arch)"
  os="$(detect_os)"
  ext="tar.gz"
  printf '%s' "${BINARY}-PLACEHOLDER-${arch}-${os}.${ext}"
}

resolve_asset_name() {
  api_url="$(release_api_url)"
  pattern="$(asset_name | sed "s/PLACEHOLDER/[^\"]*/")"
  json="$(sh -c "$DOWNLOADER \"$api_url\"")"
  name="$(printf '%s' "$json" | tr ',' '\n' | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | grep -E "^${pattern}$" | head -n 1 || true)"

  if [ -z "$name" ]; then
    echo "No release asset found for $(detect_arch)-$(detect_os)" >&2
    exit 1
  fi

  printf '%s' "$name"
}

main() {
  ensure_downloader
  asset="$(resolve_asset_name)"
  if [ "$VERSION" = "latest" ]; then
    url="https://github.com/$REPO/releases/latest/download/$asset"
  else
    url="https://github.com/$REPO/releases/download/$VERSION/$asset"
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT INT TERM

  echo "Downloading $url"
  sh -c "$DOWNLOADER \"$url\"" > "$tmpdir/$asset"

  mkdir -p "$INSTALL_DIR"
  tar -xzf "$tmpdir/$asset" -C "$tmpdir"
  install "$tmpdir/$BINARY" "$INSTALL_DIR/$BINARY"

  echo "Installed $BINARY to $INSTALL_DIR/$BINARY"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      echo "Warning: $INSTALL_DIR is not currently in PATH" >&2
      ;;
  esac
}

main "$@"
