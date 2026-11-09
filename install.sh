#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
PRIMARY_CMD="agychat"
APPS_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
TARGET_BIN="$SCRIPT_DIR/target/release/agychat"

echo "=== Instalando Antigravity Chat Manager (agychat) en Rust + Slint ==="

# 1. Detectar binario precompilado o compilar en release
if [ -f "$SCRIPT_DIR/agychat" ]; then
    TARGET_BIN="$SCRIPT_DIR/agychat"
    echo "📦 Utilizando binario precompilado: $TARGET_BIN"
elif [ -f "$SCRIPT_DIR/target/release/agychat" ]; then
    TARGET_BIN="$SCRIPT_DIR/target/release/agychat"
else
    echo "🔨 Compilando binario optimizado en release (cargo build --release)..."
    (cd "$SCRIPT_DIR" && cargo build --release)
    TARGET_BIN="$SCRIPT_DIR/target/release/agychat"
fi

# 2. Asegurar permisos de ejecución
chmod +x "$TARGET_BIN"
if [ -f "$SCRIPT_DIR/uninstall.sh" ]; then
    chmod +x "$SCRIPT_DIR/uninstall.sh"
fi

# 3. Crear directorios necesarios
mkdir -p "$BIN_DIR" "$APPS_DIR" "$ICONS_DIR"

# 4. Instalar binario principal 'agychat'
ln -sf "$TARGET_BIN" "$BIN_DIR/$PRIMARY_CMD"

# 5. Crear alias secundarios para compatibilidad
ln -sf "$TARGET_BIN" "$BIN_DIR/duplicate-chat-antigravity"
ln -sf "$TARGET_BIN" "$BIN_DIR/antigravity-chat"

# Limpiar enlaces obsoletos
rm -f "$BIN_DIR/duplicate-chat"

echo "✅ Binario nativo instalado: $BIN_DIR/$PRIMARY_CMD -> $TARGET_BIN"
echo "   (Alias creados: antigravity-chat, duplicate-chat-antigravity)"

# 6. Instalar Icono de la Aplicación
if [ -f "$SCRIPT_DIR/agychat.svg" ]; then
    cp "$SCRIPT_DIR/agychat.svg" "$ICONS_DIR/agychat.svg"
    echo "✅ Icono instalado en: $ICONS_DIR/agychat.svg"
fi

# 7. Crear archivo .desktop para el Menú de Aplicaciones y Gestor de Tareas
cat <<EOF > "$APPS_DIR/agychat.desktop"
[Desktop Entry]
Version=1.0
Name=agychat
GenericName=Antigravity Chat Manager
Comment=Gestor de Conversaciones y Ramificación en Vivo para Google Antigravity
Exec=$BIN_DIR/agychat -g
Icon=agychat
Terminal=false
Type=Application
Categories=Development;
StartupWMClass=agychat
Keywords=antigravity;chat;duplicate;export;import;ai;slint;rust;
EOF

chmod +x "$APPS_DIR/agychat.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi
echo "✅ Lanzador de escritorio creado en: $APPS_DIR/agychat.desktop"
echo "   (Visible en el Gestor de Tareas, Dock y Menú de Aplicaciones)"

# 8. Verificar PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "⚠️  Aviso: $BIN_DIR no parece estar en tu variable PATH actual."
    echo "   Para usar '$PRIMARY_CMD' directamente, añade esto a tu ~/.bashrc o ~/.zshrc:"
    echo '   export PATH="$HOME/.local/bin:$PATH"'
else
    echo "✅ El directorio $BIN_DIR está en tu PATH."
fi

echo ""
echo "🎉 ¡Instalación completa!"
echo "Puedes usarlo desde la terminal:"
echo "   agychat -h          # Ver ayuda CLI"
echo "   agychat -l          # Listar chats disponibles"
echo "   agychat -g          # Abrir interfaz gráfica nativa Slint"
echo "O buscar 'agychat' directamente en el menú de aplicaciones de tu sistema."
