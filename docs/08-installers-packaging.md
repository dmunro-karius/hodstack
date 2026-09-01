# Installers: `install.sh`, `install.ps1`, npm package

## What's missing

The only install path locally is `cargo install --path .`, which requires
a Rust toolchain. The remote repo ships three install paths that need
nothing but `curl`/`iwr`/`npm`.

## What the remote repo does

### `install.sh` (curl installer, macOS/Linux) — full file

```sh
#!/bin/sh
set -eu

REPO="hodstack/hodstack"
TAG="${HOD_TAG:-}"
DIR="${HOD_INSTALL_DIR:-$HOME/.local/bin}"

if [ -n "$TAG" ]; then
    RELEASE="https://github.com/$REPO/releases/download/$TAG"
else
    RELEASE="https://github.com/$REPO/releases/latest/download"
fi

URL="${HOD_RELEASE_URL:-$RELEASE}"

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

target() {
    system="$(uname -s)"
    machine="$(uname -m)"
    case "$system $machine" in
        'Darwin arm64') echo 'aarch64-apple-darwin' ;;
        'Darwin x86_64') echo 'x86_64-apple-darwin' ;;
        'Linux aarch64' | 'Linux arm64') echo 'aarch64-unknown-linux-musl' ;;
        'Linux x86_64') echo 'x86_64-unknown-linux-musl' ;;
        *) fail "no build for $system $machine" ;;
    esac
}

download() {
    if command -v curl > /dev/null 2>&1; then
        curl --fail --silent --show-error --location "$1" --output "$2"
    elif command -v wget > /dev/null 2>&1; then
        wget --quiet "$1" --output-document "$2"
    else
        fail 'this computer has no curl and no wget'
    fi
}

checksum() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum > /dev/null 2>&1; then
        shasum --algorithm 256 "$1" | cut -d ' ' -f 1
    else
        fail 'this computer has no sha256sum and no shasum'
    fi
}

on_path() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

TARGET="$(target)"
FILE="hod-$TARGET.tar.gz"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

printf 'Downloading hod for %s\n' "$TARGET"
download "$URL/$FILE" "$WORK/$FILE"
download "$URL/checksums.txt" "$WORK/checksums.txt"

WANTED="$(grep " $FILE\$" "$WORK/checksums.txt" | cut -d ' ' -f 1)"
FOUND="$(checksum "$WORK/$FILE")"
[ -n "$WANTED" ] || fail "checksums.txt names no $FILE"
[ "$WANTED" = "$FOUND" ] || fail "the checksum of $FILE does not agree with checksums.txt"

tar -xzf "$WORK/$FILE" -C "$WORK" hod
mkdir -p "$DIR"
install -m 755 "$WORK/hod" "$DIR/hod" 2> /dev/null || {
    cp "$WORK/hod" "$DIR/hod"
    chmod 755 "$DIR/hod"
}

printf 'Installed %s\n' "$("$DIR/hod" --version)"
printf '         %s\n' "$DIR/hod"

if ! on_path "$DIR"; then
    printf '\nAdd it to your PATH:\n'
    printf '  export PATH="%s:$PATH"\n' "$DIR"
fi
```

Usage (from the release notes template in `12-ci-cd-and-repo-docs.md`):

```sh
curl -fsSL https://github.com/hodstack/hodstack/releases/latest/download/install.sh | sh
```

### `install.ps1` (Windows) — full file

```powershell
#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

$repo = 'hodstack/hodstack'
$dir = if ($env:HOD_INSTALL_DIR) { $env:HOD_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'hod\bin' }
$url = if ($env:HOD_TAG) {
    "https://github.com/$repo/releases/download/$env:HOD_TAG"
} else {
    "https://github.com/$repo/releases/latest/download"
}

function Get-Target {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64-pc-windows-msvc' }
        default { throw "no build for $env:PROCESSOR_ARCHITECTURE" }
    }
}

$target = Get-Target
$file = "hod-$target.tar.gz"
$work = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ([System.Guid]::NewGuid()))

try {
    Write-Host "Downloading hod for $target"
    Invoke-WebRequest -Uri "$url/$file" -OutFile (Join-Path $work $file) -UseBasicParsing
    Invoke-WebRequest -Uri "$url/checksums.txt" -OutFile (Join-Path $work 'checksums.txt') -UseBasicParsing

    $line = Select-String -Path (Join-Path $work 'checksums.txt') -Pattern " $file$"
    if (-not $line) { throw "checksums.txt names no $file" }

    $wanted = $line.Line.Split(' ')[0]
    $found = (Get-FileHash -Path (Join-Path $work $file) -Algorithm SHA256).Hash.ToLower()
    if ($wanted -ne $found) { throw "the checksum of $file does not agree with checksums.txt" }

    tar -xzf (Join-Path $work $file) -C $work hod.exe
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Move-Item -Path (Join-Path $work 'hod.exe') -Destination (Join-Path $dir 'hod.exe') -Force

    Write-Host "Installed $(& (Join-Path $dir 'hod.exe') --version)"
    Write-Host "          $(Join-Path $dir 'hod.exe')"

    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($path -notlike "*$dir*") {
        [Environment]::SetEnvironmentVariable('Path', "$dir;$path", 'User')
        Write-Host ''
        Write-Host "Added $dir to your PATH. Open a new terminal."
    }
}
finally {
    Remove-Item -Path $work -Recurse -Force -ErrorAction SilentlyContinue
}
```

Usage:

```powershell
irm https://github.com/hodstack/hodstack/releases/latest/download/install.ps1 | iex
```

### npm package (`npm/`)

`npm/package.json`:

```json
{
  "name": "hodstack",
  "version": "0.0.0",
  "description": "Run a Hodstack skill with a coding agent from your terminal.",
  "homepage": "https://github.com/hodstack/hodstack",
  "repository": { "type": "git", "url": "git+https://github.com/hodstack/hodstack.git", "directory": "npm" },
  "license": "MIT",
  "author": "Nuno Maduro <enunomaduro@gmail.com>",
  "keywords": ["agent", "skills", "cli", "claude", "codex"],
  "bin": { "hod": "bin/hod.js" },
  "files": ["bin/hod.js", "install.js"],
  "scripts": { "postinstall": "node install.js" },
  "engines": { "node": ">=20" },
  "os": ["darwin", "linux", "win32"],
  "cpu": ["x64", "arm64"]
}
```

`npm/install.js` — a `postinstall` script that downloads the right platform
tarball (same checksum-verify-then-extract pattern as `install.sh`), no
build step, no native compilation:

```js
const { createHash } = require('node:crypto')
const { execFileSync } = require('node:child_process')
const { chmodSync, copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')

const REPOSITORY = 'hodstack/hodstack'
const TAG = process.env.HOD_TAG
const RELEASE = TAG
  ? `https://github.com/${REPOSITORY}/releases/download/${TAG}`
  : `https://github.com/${REPOSITORY}/releases/latest/download`

const TARGETS = {
  'darwin arm64': 'aarch64-apple-darwin',
  'darwin x64': 'x86_64-apple-darwin',
  'linux arm64': 'aarch64-unknown-linux-musl',
  'linux x64': 'x86_64-unknown-linux-musl',
  'win32 x64': 'x86_64-pc-windows-msvc',
}

function target() {
  const platform = `${process.platform} ${process.arch}`
  const found = TARGETS[platform]
  if (!found) throw new Error(`no build for ${platform}`)
  return found
}

async function download(address) {
  const response = await fetch(address)
  if (!response.ok) throw new Error(`${address} gives ${response.status}`)
  return Buffer.from(await response.arrayBuffer())
}

function verify(archive, checksums, file) {
  const line = checksums.toString().split('\n').find((row) => row.endsWith(` ${file}`))
  if (!line) throw new Error(`checksums.txt names no ${file}`)

  const wanted = line.split(' ')[0]
  const found = createHash('sha256').update(archive).digest('hex')
  if (wanted !== found) throw new Error(`the checksum of ${file} does not agree with checksums.txt`)
}

async function install() {
  const name = target()
  const file = `hod-${name}.tar.gz`
  const binary = process.platform === 'win32' ? 'hod.exe' : 'hod'

  const [archive, checksums] = await Promise.all([
    download(`${RELEASE}/${file}`),
    download(`${RELEASE}/checksums.txt`),
  ])

  verify(archive, checksums, file)

  const work = mkdtempSync(join(tmpdir(), 'hodstack-'))
  try {
    writeFileSync(join(work, file), archive)
    execFileSync('tar', ['-xzf', join(work, file), '-C', work, binary])

    const directory = join(__dirname, 'bin')
    mkdirSync(directory, { recursive: true })
    copyFileSync(join(work, binary), join(directory, binary))
    chmodSync(join(directory, binary), 0o755)
  } finally {
    rmSync(work, { recursive: true, force: true })
  }
}

install().catch((error) => {
  process.stderr.write(`error: ${error.message}\n`)
  process.exitCode = 1
})
```

`npm/bin/hod.js` — thin exec shim so `npx hod` / a locally-installed `hod`
bin just forwards to the downloaded native binary:

```js
#!/usr/bin/env node

const { spawnSync } = require('node:child_process')
const { existsSync } = require('node:fs')
const { join } = require('node:path')

const binary = join(__dirname, process.platform === 'win32' ? 'hod.exe' : 'hod')

if (!existsSync(binary)) {
  process.stderr.write('error: the hod binary is absent. Run `npm rebuild hodstack`.\n')
  process.exit(1)
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' })
process.exit(result.status === null ? 1 : result.status)
```

Usage: `npm install --global hodstack@edge`.

## Design points worth keeping

- **All three installers implement the identical checksum-verify pattern**
  independently (shell `sha256sum`/`shasum`, PowerShell `Get-FileHash`,
  Node `crypto.createHash`) — no shared code, but the *shape* (download
  archive + `checksums.txt`, look up the wanted line, compare, refuse to
  install on mismatch) must stay identical across all three, since they all
  consume the same release artifacts (see `04-update-command.md` and
  `12-ci-cd-and-repo-docs.md`'s release workflow, which is what actually
  produces `hod-<target>.tar.gz` + `checksums.txt` + `version.txt`).
- **`HOD_INSTALL_DIR`, `HOD_TAG`, `HOD_RELEASE_URL`** env vars are
  consistent override points across `install.sh` and `install.ps1` (npm's
  `install.js` only respects `HOD_TAG`, since npm's own install directory
  isn't user-choosable the same way).
- **Default install dir**: `~/.local/bin` (Unix), `%LOCALAPPDATA%\hod\bin`
  (Windows) — both are user-writable without sudo/admin, and both scripts
  check whether that directory is already on `PATH` and tell the user how
  to add it if not (rather than silently editing shell rc files).
- npm intentionally ships **no native addon / no build step** — it's a pure
  postinstall downloader, which is what keeps `os`/`cpu` fields in
  `package.json` accurate as a filter and keeps `npm install` fast.

## Implementation notes for this repo

- This is independent of the Rust-side features (docs 1–7) but **depends
  on the release pipeline producing the exact artifact names and layout**
  these scripts expect: `hod-<target>.tar.gz`, `checksums.txt`,
  `version.txt`, published together at one release URL. See
  `12-ci-cd-and-repo-docs.md`'s `release.yml` for how the remote produces
  these.
- If this repo isn't ready to stand up GitHub Releases + CI yet, these
  installers are not independently useful — sequence this after
  `12-ci-cd-and-repo-docs.md`, or at least after deciding on a release
  process.
- No Rust-side dependency changes — this is pure shell/PowerShell/Node.
