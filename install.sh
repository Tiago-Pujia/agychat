#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_SCRIPT="$SCRIPT_DIR/duplicate_chat.py"
BIN_DIR="$HOME/.local/bin"
PRIMARY_CMD="agychat"

echo "=== Instalando Antigravity Chat Manager (agychat) ==="

# 1. Asegurar permisos de ejecución en los scripts
chmod +x "$TARGET_SCRIPT" "$SCRIPT_DIR/uninstall.sh"

# 2. Crear ~/.local/bin si no existe
mkdir -p "$BIN_DIR"

# 3. Crear enlace simbólico principal 'agychat'
ln -sf "$TARGET_SCRIPT" "$BIN_DIR/$PRIMARY_CMD"

# 4. Crear alias secundarios para compatibilidad
ln -sf "$TARGET_SCRIPT" "$BIN_DIR/duplicate-chat-antigravity"
ln -sf "$TARGET_SCRIPT" "$BIN_DIR/antigravity-chat"

# Limpiar enlaces obsoletos
rm -f "$BIN_DIR/duplicate-chat"

echo "✅ Comando principal instalado: $BIN_DIR/$PRIMARY_CMD -> $TARGET_SCRIPT"
echo "   (Alias creados: antigravity-chat, duplicate-chat-antigravity)"

# 5. Verificar PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "⚠️  Aviso: $BIN_DIR no parece estar en tu variable PATH actual."
    echo "   Para usar '$PRIMARY_CMD' directamente, añade esto a tu ~/.bashrc o ~/.zshrc:"
    echo '   export PATH="$HOME/.local/bin:$PATH"'
else
    echo "✅ El directorio $BIN_DIR está en tu PATH."
fi

echo ""
echo "🎉 ¡Instalación completa!"
echo "Prueba ejecutar:"
echo "   agychat -h"
