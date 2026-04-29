#!/bin/bash
# Repository and artifact settings
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"
# Initialize variable to decide whether to accept pre-releases
UNSTABLE=false
# 1. Process arguments (flags)
for arg in "$@"; do
    case $arg in
        --unstable)
            UNSTABLE=true
            shift
            ;;
    esac
done
echo "Fetching latest version..."
# 2. Define GitHub API URL
# If unstable is true, fetch the first release from the general list (may be a pre-release)
# Otherwise, use the 'latest' endpoint which guarantees a stable release
if [ "$UNSTABLE" = true ]; then
    echo "UNSTABLE mode enabled: Including pre-releases in search."
    API_URL="https://api.github.com/repos/$REPO/releases"
    # From the list endpoint, grab the first item containing the desired binary
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   head -n 1 | \
                   cut -d '"' -f 4)
else
    API_URL="https://api.github.com/repos/$REPO/releases/latest"
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   cut -d '"' -f 4)
fi
# Validate whether the URL was extracted correctly
if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not locate $BINARY_NAME in repository $REPO."
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
