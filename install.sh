#!/bin/bash

# Repository and artifact configuration
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"

# 1. Get the download URL of the latest release via GitHub API
DOWNLOAD_URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest | \
               grep "browser_download_url" | \
               grep "$BINARY_NAME" | \
               cut -d '"' -f 4)

# Validate if the URL was extracted correctly
if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not locate file $BINARY_NAME in the latest release of $REPO."
    exit 1
fi

# 2. Download the binary
echo "Downloading from: $DOWNLOAD_URL"
curl -L -o "$BINARY_NAME" "$DOWNLOAD_URL"

if [ $? -ne 0 ]; then
    echo "Error during download."
    exit 1
fi

# 3. Set execute permissions
chmod +x "$BINARY_NAME"

# 4. Move the binary to the destination directory (requires superuser privileges)
echo "Installing to $INSTALL_PATH..."
sudo mv "$BINARY_NAME" "$INSTALL_PATH"

# 5. Final verification
if [ $? -eq 0 ]; then
    echo "Installation completed successfully."
else
    echo "Error during installation to $INSTALL_PATH."
    exit 1
fi
