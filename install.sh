#!/bin/bash
set -e

# Repository and artifact settings
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"

# Initialize variables
REAL_TIME=false
SPECIFIC_VERSION=""
USER_MODE=false

# Helper for elevated commands
run_elevated() {
    if [ "$EUID" -eq 0 ]; then
        "$@"
    else
        sudo "$@"
    fi
}

# 1. Process arguments (flags)
while [[ $# -gt 0 ]]; do
    case "$1" in
        --real-time)
            REAL_TIME=true
            shift
            ;;
        --unstable)
            echo "Warning: --unstable is deprecated. Redirecting to --real-time channel."
            REAL_TIME=true
            shift
            ;;
        --user)
            USER_MODE=true
            INSTALL_PATH="$HOME/.local/bin/nedit"
            shift
            ;;
        --version)
            if [[ -n "$2" && "$2" != -* ]]; then
                SPECIFIC_VERSION="$2"
                shift 2
            else
                echo "Error: --version requires a valid version argument (e.g., --version 0.5.0)."
                exit 1
            fi
            ;;
        *)
            echo "Unknown argument: $1"
            shift
            ;;
    esac
done

echo "Fetching NEdit..."

# 2. Define Download URL and ref for assets
REF="main"

if [ "$REAL_TIME" = true ]; then
    if [ -n "$SPECIFIC_VERSION" ]; then
        echo "Error: Cannot mix --real-time (nightly) with a specific --version."
        exit 1
    fi
    echo "Installing NEdit Real-time (Bleeding Edge)..."
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/nightly/nedit_linux"
    REF="main"

elif [ -n "$SPECIFIC_VERSION" ]; then
    TAG_VERSION="$SPECIFIC_VERSION"
    REF="$TAG_VERSION"
    
    echo "Fetching specific version: $TAG_VERSION..."
    API_URL="https://api.github.com/repos/$REPO/releases/tags/$TAG_VERSION"
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   cut -d '"' -f 4)
else
    echo "Fetching latest stable version..."
    API_URL="https://api.github.com/repos/$REPO/releases/latest"
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   cut -d '"' -f 4)
    LATEST_TAG=$(curl -s "$API_URL" | grep '"tag_name":' | head -n 1 | cut -d '"' -f 4)
    if [ -n "$LATEST_TAG" ]; then
        REF="$LATEST_TAG"
    fi
fi

# Validate whether the URL was extracted correctly
if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not locate $BINARY_NAME for the requested version."
    echo "Please check the version string, your internet connection, or repository $REPO."
    exit 1
fi

# 3. Download the binary
echo "Downloading from: $DOWNLOAD_URL"
curl -L -o "$BINARY_NAME" "$DOWNLOAD_URL"
if [ $? -ne 0 ]; then
    echo "Error during download."
    exit 1
fi

# 4. Set execution permissions
chmod +x "$BINARY_NAME"

# 5. Move binary to destination directory
echo "Installing to $INSTALL_PATH..."
if [ "$USER_MODE" = true ]; then
    mkdir -p "$(dirname "$INSTALL_PATH")"
    mv "$BINARY_NAME" "$INSTALL_PATH"
else
    run_elevated mkdir -p "$(dirname "$INSTALL_PATH")"
    run_elevated mv "$BINARY_NAME" "$INSTALL_PATH"
fi

# 6. Prepare desktop entry and icon
TEMP_DIR=""
cleanup() {
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
DESKTOP_SRC=""
ICON_SRC=""

# Check local files first
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/nedit.desktop" ]; then
    DESKTOP_SRC="$SCRIPT_DIR/nedit.desktop"
fi
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/assets/nedit.png" ]; then
    ICON_SRC="$SCRIPT_DIR/assets/nedit.png"
fi

# Download missing files if needed
if [ -z "$DESKTOP_SRC" ] || [ -z "$ICON_SRC" ]; then
    TEMP_DIR=$(mktemp -d)
    
    if [ -z "$DESKTOP_SRC" ]; then
        echo "Downloading desktop entry..."
        if curl -fsSL "https://raw.githubusercontent.com/$REPO/$REF/nedit.desktop" -o "$TEMP_DIR/nedit.desktop" 2>/dev/null; then
            DESKTOP_SRC="$TEMP_DIR/nedit.desktop"
        elif curl -fsSL "https://raw.githubusercontent.com/$REPO/main/nedit.desktop" -o "$TEMP_DIR/nedit.desktop" 2>/dev/null; then
            DESKTOP_SRC="$TEMP_DIR/nedit.desktop"
        else
            cat << 'EOF' > "$TEMP_DIR/nedit.desktop"
[Desktop Entry]
Name=NEdit
GenericName=Text Editor
GenericName[pt_BR]=Editor de Texto
Comment=A modern, lightweight terminal text editor written in Rust
Comment[pt_BR]=Editor de texto moderno e leve para terminal em Rust
TryExec=nedit
Exec=nedit %F
Icon=nedit
Terminal=false
Type=Application
Categories=Utility;Development;TextEditor;
Keywords=text;editor;syntax;terminal;rust;
StartupNotify=false
MimeType=text/plain;text/markdown;text/english;text/x-rust;text/x-c;text/x-c++;text/x-csrc;text/x-chdr;text/x-python;application/json;application/toml;application/xml;text/x-lua;text/x-yaml;text/javascript;application/javascript;application/x-shellscript;text/x-shellscript;
EOF
            DESKTOP_SRC="$TEMP_DIR/nedit.desktop"
        fi
    fi

    if [ -z "$ICON_SRC" ]; then
        echo "Downloading icon..."
        if curl -fsSL "https://raw.githubusercontent.com/$REPO/$REF/assets/nedit.png" -o "$TEMP_DIR/nedit.png" 2>/dev/null; then
            ICON_SRC="$TEMP_DIR/nedit.png"
        elif curl -fsSL "https://raw.githubusercontent.com/$REPO/main/assets/nedit.png" -o "$TEMP_DIR/nedit.png" 2>/dev/null; then
            ICON_SRC="$TEMP_DIR/nedit.png"
        fi
    fi
fi

# 7. Install desktop entry and icon
if [ "$USER_MODE" = true ]; then
    DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"

    if [ -n "$DESKTOP_SRC" ] && [ -f "$DESKTOP_SRC" ]; then
        mkdir -p "$DESKTOP_DIR"
        cp "$DESKTOP_SRC" "$DESKTOP_DIR/nedit.desktop"
        chmod 644 "$DESKTOP_DIR/nedit.desktop"
        command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    fi

    if [ -n "$ICON_SRC" ] && [ -f "$ICON_SRC" ]; then
        mkdir -p "$ICON_DIR"
        mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/icons"
        cp "$ICON_SRC" "$ICON_DIR/nedit.png"
        cp "$ICON_SRC" "${XDG_DATA_HOME:-$HOME/.local/share}/icons/nedit.png"
        chmod 644 "$ICON_DIR/nedit.png" "${XDG_DATA_HOME:-$HOME/.local/share}/icons/nedit.png"
        command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" 2>/dev/null || true
    fi
else
    SYS_DESKTOP_DIR="/usr/local/share/applications"
    SYS_ICON_DIR="/usr/local/share/icons/hicolor/256x256/apps"
    SYS_PIXMAP_DIR="/usr/share/pixmaps"

    if [ -n "$DESKTOP_SRC" ] && [ -f "$DESKTOP_SRC" ]; then
        run_elevated mkdir -p "$SYS_DESKTOP_DIR"
        run_elevated cp "$DESKTOP_SRC" "$SYS_DESKTOP_DIR/nedit.desktop"
        run_elevated chmod 644 "$SYS_DESKTOP_DIR/nedit.desktop"
        command -v update-desktop-database >/dev/null 2>&1 && run_elevated update-desktop-database "$SYS_DESKTOP_DIR" 2>/dev/null || true
    fi

    if [ -n "$ICON_SRC" ] && [ -f "$ICON_SRC" ]; then
        run_elevated mkdir -p "$SYS_ICON_DIR" "$SYS_PIXMAP_DIR"
        run_elevated cp "$ICON_SRC" "$SYS_ICON_DIR/nedit.png"
        run_elevated cp "$ICON_SRC" "$SYS_PIXMAP_DIR/nedit.png"
        run_elevated chmod 644 "$SYS_ICON_DIR/nedit.png" "$SYS_PIXMAP_DIR/nedit.png"
        command -v gtk-update-icon-cache >/dev/null 2>&1 && run_elevated gtk-update-icon-cache "/usr/local/share/icons/hicolor" 2>/dev/null || true
        command -v gtk-update-icon-cache >/dev/null 2>&1 && run_elevated gtk-update-icon-cache "/usr/share/icons/hicolor" 2>/dev/null || true
    fi

    # Also register for the calling user if running under sudo
    if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
        USER_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
        if [ -n "$USER_HOME" ] && [ -d "$USER_HOME" ]; then
            U_DESKTOP_DIR="$USER_HOME/.local/share/applications"
            U_ICON_DIR="$USER_HOME/.local/share/icons/hicolor/256x256/apps"
            
            if [ -n "$DESKTOP_SRC" ] && [ -f "$DESKTOP_SRC" ]; then
                mkdir -p "$U_DESKTOP_DIR"
                cp "$DESKTOP_SRC" "$U_DESKTOP_DIR/nedit.desktop"
                chown "$SUDO_USER:" "$U_DESKTOP_DIR/nedit.desktop" 2>/dev/null || true
                chmod 644 "$U_DESKTOP_DIR/nedit.desktop"
                command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$U_DESKTOP_DIR" 2>/dev/null || true
            fi
            
            if [ -n "$ICON_SRC" ] && [ -f "$ICON_SRC" ]; then
                mkdir -p "$U_ICON_DIR"
                cp "$ICON_SRC" "$U_ICON_DIR/nedit.png"
                chown -R "$SUDO_USER:" "$USER_HOME/.local/share/icons" 2>/dev/null || true
                chmod 644 "$U_ICON_DIR/nedit.png"
            fi
        fi
    fi
fi

# 8. Final check
if [ $? -eq 0 ]; then
    echo "Installation completed successfully."
else
    echo "Error during installation to $INSTALL_PATH."
    exit 1
fi
