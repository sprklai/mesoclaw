#!/bin/bash
# Script to check Stronghold vault location and contents

echo "=== Checking Stronghold Vault Location ==="
echo ""

# Linux vault location
LINUX_VAULT_DIR="$HOME/.local/share/com.aiboilerplate.credentials"
echo "Linux vault directory: $LINUX_VAULT_DIR"

if [ -d "$LINUX_VAULT_DIR" ]; then
    echo "✓ Directory exists"
    echo ""
    echo "Files in vault directory:"
    ls -lah "$LINUX_VAULT_DIR"
    echo ""

    if [ -f "$LINUX_VAULT_DIR/secrets.hold" ]; then
        echo "✓ secrets.hold exists"
        echo "  Size: $(stat -f%z "$LINUX_VAULT_DIR/secrets.hold" 2>/dev/null || stat -c%s "$LINUX_VAULT_DIR/secrets.hold")"
        echo "  Modified: $(stat -f%Sm "$LINUX_VAULT_DIR/secrets.hold" 2>/dev/null || stat -c%y "$LINUX_VAULT_DIR/secrets.hold")"
    else
        echo "✗ secrets.hold does NOT exist"
    fi

    if [ -f "$LINUX_VAULT_DIR/salt.txt" ]; then
        echo "✓ salt.txt exists"
        echo "  Size: $(stat -f%z "$LINUX_VAULT_DIR/salt.txt" 2>/dev/null || stat -c%s "$LINUX_VAULT_DIR/salt.txt")"
    else
        echo "✗ salt.txt does NOT exist (this is expected after fix)"
    fi
else
    echo "✗ Directory does NOT exist"
fi

echo ""
echo "=== Instructions ==="
echo "If you see an OLD secrets.hold file (modified before today), you MUST delete it:"
echo "  rm -rf $LINUX_VAULT_DIR/secrets.hold"
echo "  rm -rf $LINUX_VAULT_DIR/salt.txt"
echo ""
echo "Then restart the app to create a new vault with correct encryption."
