#!/bin/sh
# Jzen — JSON Editor installer
# Usage: curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh
# Or with options:
#   INSTALL_DIR=/path/to/bin curl -fsSL ... | sh
#   SKIP_COMPLETIONS=1 curl -fsSL ... | sh  # skip completions

set -e

REPO="caoergou/jzen"
BIN_NAME="jzen"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
SKIP_COMPLETIONS="${SKIP_COMPLETIONS:-0}"

detect_shell() {
    if [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    elif [ -n "$BASH_VERSION" ]; then
        echo "bash"
    elif [ -n "$FISH_VERSION" ]; then
        echo "fish"
    else
        # Try to detect from SHELL env
        case "${SHELL:-}" in
            */zsh) echo "zsh" ;;
            */bash) echo "bash" ;;
            */fish) echo "fish" ;;
            *) echo "" ;;
        esac
    fi
}

install_completions() {
    shell="$1"
    jzen_path="$INSTALL_DIR/$BIN_NAME"

    case "$shell" in
        bash)
            # Try multiple locations
            for dir in "$HOME/.local/share/bash-completion/completions" "$HOME/.bash_completion.d"; do
                if [ -d "$dir" ] || mkdir -p "$dir" 2>/dev/null; then
                    $jzen_path completions bash > "$dir/$BIN_NAME"
                    echo "✓ Bash completions installed to $dir/$BIN_NAME"
                    return 0
                fi
            done
            # Fallback: output to stdout with instructions
            echo "Could not auto-install bash completions. Run manually:"
            echo "  $jzen_path completions bash > ~/.bash_completion.d/$BIN_NAME"
            ;;
        zsh)
            # Try fpath directories
            for dir in "$HOME/.zfunc" "$HOME/.local/share/zsh/site-functions"; do
                if [ -d "$dir" ] || mkdir -p "$dir" 2>/dev/null; then
                    $jzen_path completions zsh > "$dir/_$BIN_NAME"
                    echo "✓ Zsh completions installed to $dir/_$BIN_NAME"
                    # Check if fpath is configured
                    if [ -f "$HOME/.zshrc" ] && ! grep -q "fpath=.*\.zfunc" "$HOME/.zshrc" 2>/dev/null; then
                        echo "  Add to ~/.zshrc: fpath=(~/.zfunc \$fpath)"
                    fi
                    return 0
                fi
            done
            echo "Could not auto-install zsh completions. Run manually:"
            echo "  mkdir -p ~/.zfunc"
            echo "  $jzen_path completions zsh > ~/.zfunc/_$BIN_NAME"
            ;;
        fish)
            dir="$HOME/.config/fish/completions"
            mkdir -p "$dir" 2>/dev/null || true
            if [ -d "$dir" ]; then
                $jzen_path completions fish > "$dir/$BIN_NAME.fish"
                echo "✓ Fish completions installed to $dir/$BIN_NAME.fish"
                return 0
            fi
            echo "Could not auto-install fish completions. Run manually:"
            echo "  $jzen_path completions fish > ~/.config/fish/completions/$BIN_NAME.fish"
            ;;
        *)
            echo "Auto-install not supported for this shell. Run manually:"
            echo "  $jzen_path completions <bash|zsh|fish> > <appropriate-location>"
            ;;
    esac
}

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  ASSET="jzen-linux-x86_64" ;;
      aarch64) ASSET="jzen-linux-aarch64" ;;
      *)       echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64) ASSET="jzen-macos-x86_64" ;;
      arm64)  ASSET="jzen-macos-aarch64" ;;
      *)      echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    echo "For Windows, download jzen-windows-x86_64.exe from:"
    echo "  https://github.com/$REPO/releases/latest"
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/latest/download/$ASSET"

echo "Downloading $ASSET..."
curl -fsSL "$URL" -o "/tmp/$BIN_NAME"
chmod +x "/tmp/$BIN_NAME"

# Install (try with sudo if needed)
if [ -w "$INSTALL_DIR" ]; then
    mv "/tmp/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "/tmp/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
fi

echo "✓ Installed: $(which $BIN_NAME)"
"$BIN_NAME" --version
echo ""

# Install completions (unless skipped)
if [ "$SKIP_COMPLETIONS" != "1" ]; then
    SHELL="$(detect_shell)"
    if [ -n "$SHELL" ]; then
        echo "Detected shell: $SHELL"
        install_completions "$SHELL"
    else
        echo "Could not detect shell. Skipping completions."
        echo "To install manually: jzen completions <bash|zsh|fish>"
    fi
fi

echo ""
echo "Done! Run 'jzen --help' to get started."
