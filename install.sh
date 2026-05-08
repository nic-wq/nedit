#!/bin/bash
# Repository and artifact settings
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"

# Initialize variables
REAL_TIME=false

# 1. Process arguments (flags)
for arg in "$@"; do
    case $arg in
        --real-time)
            REAL_TIME=true
            shift
            ;;
        --unstable)
            echo "Warning: --unstable is deprecated. Redirecting to --real-time channel."
            REAL_TIME=true
            shift
            ;;
    esac
done

echo "Fetching NEdit..."

# 2. Define Download URL
if [ "$REAL_TIME" = true ]; then
    echo "Installing NEdit Real-time (Bleeding Edge)..."
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/nightly/nedit_linux"
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
    echo "Error: Could not locate $BINARY_NAME. Please check your internet connection or repository $REPO."
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
