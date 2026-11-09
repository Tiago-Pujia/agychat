#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Empaquetando agychat como AppImage y Tarball ==="

# 1. Asegurar binario en release
if [ ! -f "target/release/agychat" ]; then
    echo "🔨 Compilando binario en release..."
    cargo build --release
fi

# 2. Generar PNG si no existe
if [ ! -f "agychat.png" ]; then
    if command -v ksvgtopng >/dev/null 2>&1; then
        ksvgtopng 256 256 agychat.svg agychat.png
    elif command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w 256 -h 256 agychat.svg -o agychat.png
    fi
fi

# 3. Descargar appimagetool si no existe
APPIMAGETOOL="/tmp/appimagetool-x86_64.AppImage"
if [ ! -f "$APPIMAGETOOL" ]; then
    echo "⬇️ Descargando appimagetool..."
    wget -q https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage -O "$APPIMAGETOOL"
    chmod +x "$APPIMAGETOOL"
fi

# 4. Preparar estructura AppDir
APPDIR="/tmp/agychat.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/icons/hicolor/scalable/apps"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/agychat "$APPDIR/usr/bin/"
cp agychat.svg "$APPDIR/usr/share/icons/hicolor/scalable/apps/"
cp agychat.svg "$APPDIR/agychat.svg"

if [ -f "agychat.png" ]; then
    cp agychat.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/"
    cp agychat.png "$APPDIR/agychat.png"
    cp agychat.png "$APPDIR/.DirIcon"
fi

cat << 'DESK' > "$APPDIR/agychat.desktop"
[Desktop Entry]
Name=agychat
GenericName=Antigravity Chat Manager
Comment=Gestor de Conversaciones y Ramificación en Vivo para Google Antigravity
Exec=agychat %F
Icon=agychat
Type=Application
Categories=Development;
StartupWMClass=agychat
Terminal=false
Keywords=antigravity;chat;duplicate;ai;rust;slint;
DESK

cat << 'RUN' > "$APPDIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"

# Si se ejecuta sin argumentos (ej: doble clic), abrir la interfaz gráfica por defecto
if [ $# -eq 0 ]; then
    exec "${HERE}/usr/bin/agychat" -g
else
    exec "${HERE}/usr/bin/agychat" "$@"
fi
RUN
chmod +x "$APPDIR/AppRun"

# 5. Generar AppImage
mkdir -p dist
ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run "$APPDIR" dist/agychat-x86_64.AppImage
chmod +x dist/agychat-x86_64.AppImage

# 6. Generar Tarball de distribución con instalador
TAR_DIR="/tmp/agychat"
rm -rf "$TAR_DIR"
mkdir -p "$TAR_DIR"
cp target/release/agychat "$TAR_DIR/"
cp install.sh "$TAR_DIR/"
cp uninstall.sh "$TAR_DIR/"
cp agychat.svg "$TAR_DIR/"
[ -f "agychat.png" ] && cp agychat.png "$TAR_DIR/"
cp README.md "$TAR_DIR/"
tar -czvf dist/agychat-linux-x86_64.tar.gz -C /tmp agychat
rm -rf "$TAR_DIR"

echo "✅ AppImage generado: dist/agychat-x86_64.AppImage"
echo "✅ Tarball generado:  dist/agychat-linux-x86_64.tar.gz"
