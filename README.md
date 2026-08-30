# Git-Release

A small utility to streamline releases. It bumps the version in your manifest file, commits the change, and creates a ready-to-push annotated git tag in one command.

> **Note:** Currently supports Rust/Cargo and Node.JS (npm, pnpm, yarn) ecosystems. More ecosystems are coming soon.

## The Problem

Releasing usually means manually bumping the version in your manifest, committing, tagging (`vX.Y.Z`), and pushing. Doing it by hand is error-prone and easy to forget.

`git-release` automates the local steps so every release is consistent.

## How It Works

Given `<kind>` = `major` | `minor` | `patch`:

1. Detects the ecosystem by looking for `Cargo.toml` or `package.json` in the target directory
2. Reads the current version from the manifest
3. Computes the next SemVer — e.g. `2.4.0` → `3.0.0` (major), `2.5.0` (minor), `2.4.1` (patch)
4. Writes the new version back to the manifest (preserving formatting)
5. Syncs the lockfile and stages all changed files:
   - **Cargo**: runs `cargo check --workspace`, stages `Cargo.toml` + `Cargo.lock`
   - **npm**: stages `package.json` + the detected lockfile (`package-lock.json`, `pnpm-lock.yaml`, or `yarn.lock`)
6. Creates a commit `Release: vX.Y.Z`
7. Creates an annotated tag `vX.Y.Z` with message `Release: vX.Y.Z` on `HEAD` — ready to push

It does **not** push. Run `git push --follow-tags` yourself.

## Requirements

- A git repository (opens `--repo` or `./` via `git2`)
- A supported manifest file at the repo root:
  - `Cargo.toml` (Rust/Cargo)
  - `package.json` (Node.JS — npm, pnpm, or yarn)
- Git config `user.name` / `user.email` set (used for the commit/tag signature)

## Installation

### Quick Install (Latest)

```bash
curl -fsSL https://raw.githubusercontent.com/StanleyMasinde/git-release/main/install.sh | sh
```

### Install a Specific Version

```bash
curl -fsSL https://raw.githubusercontent.com/StanleyMasinde/git-release/main/install.sh | sh -s v0.6.0
```

### Custom Install Directory

By default the binary is installed to `/usr/local/bin`. Set `LOC_INSTALL` to override:

```bash
curl -fsSL https://raw.githubusercontent.com/StanleyMasinde/git-release/main/install.sh | LOC_INSTALL=~/.local/bin sh
```

### Prerequisites

The installer requires:
- `curl` or `wget`
- `tar` (Linux/macOS) or `unzip` (Windows)
- `sudo` if installing to a system directory (e.g. `/usr/local/bin`)

Downloads are verified with SHA256 checksums. Installation aborts if verification fails.

### Supported Platforms

| OS | Architecture |
|----|-------------|
| Linux | x86_64, AArch64 |
| macOS | AArch64 (Apple Silicon only) |
| Windows | x86_64 |

## Usage

```text
Usage: git-release [OPTIONS] <kind>

Arguments:
  <kind>
          The release version e.g. major

          Possible values:
          - major: Bump major version (X.0.0)
          - minor: Bump minor version (x.Y.0)
          - patch: Bump patch version (x.y.Z)

Options:
  -r, --repo <PATH>
          Specify the git repo [default: ./]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

This util helps streamline the release process. Calling git release increments the tag.
```

Examples:

```bash
git-release patch          # 0.1.0 → 0.1.1 in ./Cargo.toml (or package.json) + commit + tag v0.1.1
git-release minor          # 0.1.1 → 0.2.0
git-release major          # 0.2.0 → 1.0.0
git-release patch -r /path/to/my-project
```

Then push the commit and tag:

```bash
git push --follow-tags
# or
git push && git push --tags
```

## Ideal Setup

This utility works best when your CI pipeline builds and publishes on new tags (e.g. GitHub Actions `on: push: tags: ['v*']`). Then `git-release` becomes your single local release command — bump, commit, tag, push.

## Development

```bash
cargo test   # tests SemVer bumping (get_next_version)
cargo build
```

### Adding a New Ecosystem

Each ecosystem is a struct in `src/ecosystem/recipes/` that implements the `Ecosystem` trait:

```rust
pub trait Ecosystem<'a> {
    fn new(directory: PathBuf, release_type: &'a ReleaseKind) -> Self;
    fn get_current_version(&self) -> String;
    fn bump_package_version(&self) -> (String, Vec<String>); // returns (new_version, files_to_stage)
    fn sync_lockfile(&self) -> Vec<String>;                  // returns files to stage
    // get_next_version() is provided automatically
}
```

Steps to add support for a new ecosystem:

1. Add a new file `src/ecosystem/recipes/<name>.rs` and implement the `Ecosystem` trait
2. Export it from `src/ecosystem/recipes/mod.rs`
3. Add a variant to `EcosystemType` in `src/ecosystem/types.rs` and update `EcosystemType::detect()` to recognise the manifest file
4. Handle the new variant in the `match ecosystem` block in `src/main.rs`
