# itylos

Rust CLI for sovereign ephemeral messaging with local AES-256-GCM encryption, zero-knowledge link sharing, and burn-on-read destruction via the `/api/v2/*` contract.

## Goals

- Encrypt locally before any network request.
- Never send the decryption key to the server.
- Keep the decryption key in the URL fragment `#key`.
- Validate the API contract strictly on the client side.
- Verify signed destruction proofs with Ed25519.
- Ship one clean Rust binary named `itylos`.

## Architecture

- [`src/main.rs`](src/main.rs): thin entrypoint, `run() -> anyhow::Result<()>`
- [`src/cli.rs`](src/cli.rs): `clap` parsing
- [`src/services.rs`](src/services.rs): user-facing workflows
- [`src/crypto/mod.rs`](src/crypto/mod.rs): AES-256-GCM, AAD, AAD hash, proof verification
- [`src/network/mod.rs`](src/network/mod.rs): HTTP client for `/api/v2/create_secret`, `/fetch_secret`, `/burn_secret`
- [`src/types.rs`](src/types.rs): shared request/response types and constants
- [`src/mcp/mod.rs`](src/mcp/mod.rs): MCP stdio server

Reference architecture used for repo cleanliness and release discipline:
- local clone: `C:\Users\Kerki\Desktop\itylos-cli-v2\ai-rsk-reference`

## Install

### From source

```bash
cargo install --path .
```

This installs the binary as:

```bash
itylos
```

### From GitHub Releases

Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/kerk99/itylos-cli/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/kerk99/itylos-cli/main/scripts/install.ps1 -UseBasicParsing | iex
```

Specific release:

```bash
ITYLOS_VERSION=v2.0.0 curl -fsSL https://raw.githubusercontent.com/kerk99/itylos-cli/main/scripts/install.sh | sh
```

### Local development

```bash
cargo build
cargo test
cargo build --release
```

### Windows

If `%USERPROFILE%\.cargo\bin` is in your `PATH`, you can open any terminal and run:

```powershell
itylos --help
```

Release installers place the binary in:

- Linux and macOS: `~/.local/bin/itylos`
- Windows: `%LOCALAPPDATA%\Programs\itylos\bin\itylos.exe`

After installation, open a new terminal and run `itylos`.

## Usage

```bash
itylos send "secret"
itylos send -f secret.pdf -d 24h
itylos read https://itylos.com/v/<secret_id>#<key>
itylos verify proof.json
itylos mcp
```

## API Contract

The CLI is aligned with:

- `POST /api/v2/create_secret`
- `GET /api/v2/fetch_secret?id=<secret_id>`
- `POST /api/v2/burn_secret`

Client-side checks enforced:

- `payload` must match `base64url.base64url`
- `ttl` must be `3600`, `86400`, or `604800`
- `aad_hash` must be hex-64 and consistent with `sha256(AAD(ttl))`
- `has_password` and `pwd_salt` must appear together
- fetch IDs must be hex-32
- attachment names are sanitized on extraction

## Security Model

- Zero-knowledge: the server never receives the fragment key.
- Local encryption: the cleartext is serialized and encrypted before upload.
- AAD-bound TTL: the TTL participates in authenticated encryption.
- Zeroization: sensitive buffers are cleared when possible with `zeroize`.
- Proof verification: destruction proofs are normalized and verified with Ed25519.

## Quality Gates

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build --release`

GitHub Actions workflow:
- [`.github/workflows/ci.yml`](.github/workflows/ci.yml)

Release assets on version tags:
- `itylos-vX.Y.Z-x86_64-pc-windows-msvc.zip`
- `itylos-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `itylos-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz`
- `itylos-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `itylos-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `checksums-vX.Y.Z.txt`

## Current Status

- Unit tests: passing
- Debug build: passing
- Release build: passing
- Real endpoint tests against `https://itylos.com`: target domain

## Notes

- Legacy Go and PHP files are still present for migration reference.
- The Rust binary target is now `itylos`.
