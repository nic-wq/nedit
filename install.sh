#!/bin/bash
# Repository and artifact settings
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"

# Initialize variables
REAL_TIME=false
SPECIFIC_VERSION=""

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

# 2. Define Download URL
if [ "$REAL_TIME" = true ]; then
    if [ -n "$SPECIFIC_VERSION" ]; then
        echo "Error: Cannot mix --real-time (nightly) with a specific --version."
        exit 1
    fi
    echo "Installing NEdit Real-time (Bleeding Edge)..."
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/nightly/nedit_linux"

elif [ -n "$SPECIFIC_VERSION" ]; then
    # Usamos a versão exatamente como o usuário digitou (ex: 0.5.0), já que suas tags não usam "v"
    TAG_VERSION="$SPECIFIC_VERSION"
    
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

# 5. Move binary to destination directory (requires superuser privileges)
echo "Installing to $INSTALL_PATH..."
sudo mv "$BINARY_NAME" "$INSTALL_PATH"

# 6. Final check
if [ $? -eq 0 ]; then
    echo "Installation completed successfully."
else
    echo "Error during installation to $INSTALL_PATH."
    exit 1
fi
