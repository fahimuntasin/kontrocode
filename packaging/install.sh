#!/bin/bash
set -e

INSTALL_DIR="${KONTROCODE_HOME:-/opt/kontrocode}"
BIN_DIR="/usr/local/bin"

echo "=== KontroCode Installer ==="
echo "Installing to: $INSTALL_DIR"

mkdir -p "$INSTALL_DIR" "$INSTALL_DIR/bin"

# Copy binaries
cp target/release/kontrocode-agent "$INSTALL_DIR/bin/" 2>/dev/null || echo "  skip: kontrocode-agent (not built yet)"
cp zed/target/release/kontrocode "$INSTALL_DIR/bin/" 2>/dev/null || echo "  skip: kontrocode (not built yet)"

# Create launcher
cat > "$INSTALL_DIR/bin/kontrocode-launcher" << 'LAUNCHER'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
exec "$DIR/kontrocode" "$@"
LAUNCHER
chmod +x "$INSTALL_DIR/bin/kontrocode-launcher"

# Create symlink
ln -sf "$INSTALL_DIR/bin/kontrocode-launcher" "$BIN_DIR/kontrocode"

# Desktop entry
cat > /usr/share/applications/kontrocode.desktop << 'DESKTOP'
[Desktop Entry]
Name=KontroCode
Comment=Research-first, memory-aware coding agent
Exec=/opt/kontrocode/bin/kontrocode-launcher
Icon=/opt/kontrocode/kontrocode.png
Terminal=false
Type=Application
Categories=Development;IDE;
StartupWMClass=kontrocode
DESKTOP

echo ""
echo "=== Done ==="
echo "Run: kontrocode"
echo "Agent: kontrocode-agent acp"
