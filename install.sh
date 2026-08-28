#!/usr/bin/env sh

set -e

REPO="StanleyMasinde/git-release"
BIN_NAME="git-release"
VERSION="${1:-latest}"
INSTALL_DIR="${LOC_INSTALL:-/usr/local/bin}"

detect_target() {
    local os arch

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Error: Unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    case "$os" in
        linux*)
            echo "${arch}-unknown-linux-gnu"
            ;;
        darwin*)
            if [ "$arch" != "aarch64" ]; then
                echo "Error: Intel Macs are not supported. Apple Silicon (aarch64) only." >&2
                exit 1
            fi
            echo "aarch64-apple-darwin"
            ;;
        mingw*|msys*|cygwin*)
            if [ "$arch" != "x86_64" ]; then
                echo "Error: Unsupported Windows architecture: $arch" >&2
                exit 1
            fi
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            echo "Error: Unsupported OS: $os" >&2
            exit 1
            ;;
    esac
}

get_release_data() {
    local version="$1"
    local api_url

    if [ "$version" = "latest" ]; then
        api_url="https://api.github.com/repos/$REPO/releases/latest"
    else
        api_url="https://api.github.com/repos/$REPO/releases/tags/$version"
    fi

    curl -fsSL "$api_url" || {
        echo "Error: Could not fetch release data" >&2
        exit 1
    }
}

parse_version() {
    local json="$1"
    echo "$json" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/'
}

parse_asset() {
    local json="$1"
    local filename="$2"
    local url="" digest=""

    local asset_block
    asset_block=$(echo "$json" | sed -n "/${filename}/,/\"browser_download_url\"/p")

    url=$(echo "$asset_block" | grep "browser_download_url" | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')

    digest=$(echo "$asset_block" | grep "digest" | sed 's/.*"sha256:\([^"]*\)".*/\1/')

    if [ -n "$url" ] && [ -n "$digest" ]; then
        echo "${url}|${digest}"
    fi
}

verify_checksum() {
    local file="$1"
    local expected_sha="$2"

    if [ -z "$expected_sha" ] || [ "$expected_sha" = "null" ]; then
        echo "Warning: No checksum available for this release"
        echo "Skipping verification (this is expected for older releases)"
        return 0
    fi

    echo "Verifying checksum..."

    local actual_sha
    if command -v sha256sum >/dev/null 2>&1; then
        actual_sha=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual_sha=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        echo "Warning: Neither sha256sum nor shasum found"
        echo "Cannot verify checksum"
        return 0
    fi

    if [ "$actual_sha" = "$expected_sha" ]; then
        echo "✓ Checksum verified: $expected_sha"
        return 0
    else
        echo "✗ Checksum verification failed!" >&2
        echo "  Expected: $expected_sha" >&2
        echo "  Got:      $actual_sha" >&2
        echo "" >&2
        echo "The downloaded file may be corrupted or tampered with." >&2
        echo "Please try downloading again or report this issue." >&2
        return 1
    fi
}

install_binary() {
    local version="$1"
    local target="$2"

    local ext
    case "$target" in
        *windows*) ext="zip" ;;
        *) ext="tar.gz" ;;
    esac

    local filename="${BIN_NAME}-${target}.${ext}"

    echo "${BIN_NAME} Installer"
    echo ""
    echo "Fetching release information..."

    local release_json
    release_json=$(get_release_data "$version")

    if [ "$version" = "latest" ]; then
        version=$(parse_version "$release_json")
        if [ -z "$version" ]; then
            echo "Error: Could not parse version from API response" >&2
            exit 1
        fi
    fi

    echo "Version: $version"
    echo "Target:  $target"
    echo ""

    local asset_info
    asset_info=$(parse_asset "$release_json" "$filename")

    if [ -z "$asset_info" ]; then
        echo "Error: Could not find asset '$filename' in release" >&2
        echo "" >&2
        echo "Available assets for this release:" >&2
        echo "$release_json" | grep -o '"name"[[:space:]]*:[[:space:]]*"'"${BIN_NAME}"'-[^"]*"' | sed 's/.*"'"${BIN_NAME}"'-\([^"]*\)".*/  - \1/' >&2
        exit 1
    fi

    local download_url sha256_digest
    download_url=$(echo "$asset_info" | cut -d'|' -f1)
    sha256_digest=$(echo "$asset_info" | cut -d'|' -f2)

    if [ -z "$download_url" ]; then
        echo "Error: Could not extract download URL from release data" >&2
        exit 1
    fi

    local tmp_dir
    tmp_dir=$(mktemp -d)
    cd "$tmp_dir"

    echo "Downloading from: $download_url"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --progress-bar -o "$filename" "$download_url" || {
            echo "Error: Download failed" >&2
            rm -rf "$tmp_dir"
            exit 1
        }
    elif command -v wget >/dev/null 2>&1; then
        wget -q --show-progress -O "$filename" "$download_url" || {
            echo "Error: Download failed" >&2
            rm -rf "$tmp_dir"
            exit 1
        }
    else
        echo "Error: Neither curl nor wget found" >&2
        exit 1
    fi

    echo ""

    verify_checksum "$filename" "$sha256_digest" || {
        rm -rf "$tmp_dir"
        exit 1
    }

    echo ""

    echo "Extracting..."
    case "$ext" in
        tar.gz)
            tar -xzf "$filename" || {
                echo "Error: Extraction failed" >&2
                rm -rf "$tmp_dir"
                exit 1
            }
            ;;
        zip)
            if command -v unzip >/dev/null 2>&1; then
                unzip -q "$filename" || {
                    echo "Error: Extraction failed" >&2
                    rm -rf "$tmp_dir"
                    exit 1
                }
            else
                echo "Error: unzip not found" >&2
                rm -rf "$tmp_dir"
                exit 1
            fi
            ;;
    esac

    local binary_path=""
    if [ -f "$BIN_NAME" ]; then
        binary_path="$BIN_NAME"
    elif [ -f "${BIN_NAME}.exe" ]; then
        binary_path="${BIN_NAME}.exe"
    else
        binary_path=$(find . -type f \( -name "$BIN_NAME" -o -name "${BIN_NAME}.exe" \) -print -quit 2>/dev/null || true)
    fi

    if [ -n "$binary_path" ] && [ -f "$binary_path" ]; then
        chmod +x "$binary_path" 2>/dev/null || true

        echo "Installing to $INSTALL_DIR..."

        if [ -w "$INSTALL_DIR" ]; then
            install -sm 755 "$binary_path" "$INSTALL_DIR/$BIN_NAME" || {
                echo "Error: Installation failed" >&2
                rm -rf "$tmp_dir"
                exit 1
            }
        else
            sudo install -sm 755 "$binary_path" "$INSTALL_DIR/$BIN_NAME" || {
                echo "Error: Installation failed" >&2
                echo "Make sure you have sudo privileges or set LOC_INSTALL to a writable directory" >&2
                rm -rf "$tmp_dir"
                exit 1
            }
        fi

        cd - > /dev/null
        rm -rf "$tmp_dir"

        echo ""
        echo "✓ ${BIN_NAME} was installed successfully to $INSTALL_DIR/$BIN_NAME"
        echo ""
        echo "Run '${BIN_NAME} --help' to get started"
    else
        echo "Error: Binary not found after extraction" >&2
        rm -rf "$tmp_dir"
        exit 1
    fi
}

if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    cat <<EOF
${BIN_NAME} Installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh

Or with a specific version:
  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh -s v1.4.0

Environment Variables:
  LOC_INSTALL    Installation directory (default: /usr/local/bin)

Examples:
  # Install latest version
  curl -fsSL <installer-url> | sh

  # Install specific version
  curl -fsSL <installer-url> | sh -s v1.5.0

  # Install to custom location
  curl -fsSL <installer-url> | LOC_INSTALL=~/.local/bin sh

Supported Platforms:
  - Linux (x86_64, aarch64)
  - macOS/Darwin (aarch64, Apple Silicon only)
  - Windows (x86_64)

Security:
  - Downloads are verified using SHA256 checksums from the GitHub API
  - Checksums are automatically generated by GitHub for all release assets
  - Installation will fail if checksum verification fails

Requirements:
  - curl or wget
  - sha256sum or shasum (for checksum verification)
  - tar (for Unix systems) or unzip (for Windows)
  - sudo (if installing to system directory)
EOF
    exit 0
fi

main() {
    local target version

    target=$(detect_target)

    if [ -z "$VERSION" ] || [ "$VERSION" = "latest" ]; then
        version="latest"
    else
        version="$VERSION"
    fi

    install_binary "$version" "$target"
}

main
