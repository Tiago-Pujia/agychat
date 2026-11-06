#!/usr/bin/env bash
set -e

BIN_DIR="$HOME/.local/bin"

echo "=== Desinstalando Antigravity Chat Manager (agychat) ==="

for cmd in "agychat" "antigravity-chat" "duplicate-chat-antigravity" "duplicate-chat"; do
    target="$BIN_DIR/$cmd"
    if [ -L "$target" ] || [ -f "$target" ]; then
        rm -f "$target"
        echo "✅ Comando eliminado: $target"
    fi
done

echo "Desinstalación completa."
