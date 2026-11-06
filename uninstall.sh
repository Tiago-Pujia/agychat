#!/usr/bin/env bash
set -e

BIN_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

echo "=== Desinstalando Antigravity Chat Manager (agychat) ==="

for cmd in "agychat" "antigravity-chat" "duplicate-chat-antigravity" "duplicate-chat"; do
    target="$BIN_DIR/$cmd"
    if [ -L "$target" ] || [ -f "$target" ]; then
        rm -f "$target"
        echo "✅ Comando eliminado: $target"
    fi
done

if [ -f "$APPS_DIR/agychat.desktop" ]; then
    rm -f "$APPS_DIR/agychat.desktop"
    echo "✅ Lanzador de escritorio eliminado: $APPS_DIR/agychat.desktop"
fi

if [ -f "$ICONS_DIR/agychat.svg" ]; then
    rm -f "$ICONS_DIR/agychat.svg"
    echo "✅ Icono eliminado: $ICONS_DIR/agychat.svg"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

echo "Desinstalación completa."
