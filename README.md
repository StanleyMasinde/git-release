# git-release

A small utility to streamline Rust/Cargo releases. It bumps `Cargo.toml`, commits the change, and creates a ready-to-push annotated git tag in one command.

> **Note:** Currently Rust-only (Cargo projects). Support for other ecosystems such as Node.js (`package.json`) is planned.

## The Problem

Releasing usually means manually bumping the version in `Cargo.toml`, committing, tagging (`vX.Y.Z`), and pushing. Doing it by hand is error-prone and easy to forget.

`git-release` automates the local steps so every release is consistent.

## How It Works

Given `<kind>` = `major` | `minor` | `patch`:

1. Reads `Cargo.toml` (`package.version`) in the target repo
2. Computes the next SemVer — e.g. `2.4.0` → `3.0.0` (major), `2.5.0` (minor), `2.4.1` (patch)
3. Writes the new version back to `Cargo.toml` (preserving formatting via `toml_edit`)
4. Stages `Cargo.toml` and creates a commit `Release: vX.Y.Z`
5. Creates an annotated tag `vX.Y.Z` with message `Release: vX.Y.Z` on `HEAD` — ready to push

It does **not** push. Run `git push --follow-tags` yourself.

## Requirements

- A git repository (opens `--repo` or `./` via `git2`)
- A `Cargo.toml` with `package.version` at the repo root
- Git config `user.name` / `user.email` set (used for the commit/tag signature)

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/StanleyMasinde/git-release/main/install.sh | sh
```

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
```

Examples:

```bash
git-release patch          # 0.1.0 → 0.1.1 in ./Cargo.toml + commit + tag v0.1.1
git-release minor          # 0.1.1 → 0.2.0
git-release major          # 0.2.0 → 1.0.0
git-release patch -r /path/to/my-crate
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
