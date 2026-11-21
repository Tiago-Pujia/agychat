#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

VERSION="${VERSION:-$(grep '^version = ' Cargo.toml | head -1 | cut -d '"' -f 2)}"

echo "=== Empaquetando agychat para Linux (AppImage, Debian, Fedora, Arch, Tarball) ==="

# 1. Asegurar binario en release
if [ ! -f "target/release/agychat" ]; then
    echo "🔨 Compilando binario en release..."
    cargo build --release
fi

# 2. Generar PNG si no existe
if [ ! -f "agychat.png" ]; then
    if command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w 256 -h 256 agychat.svg -o agychat.png
    elif command -v ksvgtopng >/dev/null 2>&1; then
        ksvgtopng 256 256 agychat.svg agychat.png
    fi
fi

mkdir -p dist

# 3. Empaquetar AppImage
echo "📦 [1/5] Generando AppImage..."
APPIMAGETOOL="/tmp/appimagetool-x86_64.AppImage"
if [ ! -f "$APPIMAGETOOL" ]; then
    echo "⬇️ Descargando appimagetool..."
    wget -q https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage -O "$APPIMAGETOOL" || true
    chmod +x "$APPIMAGETOOL" 2>/dev/null || true
fi

if [ -f "$APPIMAGETOOL" ] && [ -x "$APPIMAGETOOL" ]; then
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

if [ $# -eq 0 ]; then
    exec "${HERE}/usr/bin/agychat" -g
else
    exec "${HERE}/usr/bin/agychat" "$@"
fi
RUN
    chmod +x "$APPDIR/AppRun"

    ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run "$APPDIR" dist/agychat-x86_64.AppImage || true
    [ -f "dist/agychat-x86_64.AppImage" ] && chmod +x dist/agychat-x86_64.AppImage
fi

# 4. Empaquetar Tarball con instalador
echo "📦 [2/5] Generando Tarball independiente..."
TAR_DIR="/tmp/agychat_tar"
rm -rf "$TAR_DIR"
mkdir -p "$TAR_DIR"
cp target/release/agychat "$TAR_DIR/"
cp install.sh "$TAR_DIR/"
cp uninstall.sh "$TAR_DIR/"
cp agychat.svg "$TAR_DIR/"
[ -f "agychat.png" ] && cp agychat.png "$TAR_DIR/"
cp README.md "$TAR_DIR/"
tar -czf dist/agychat-linux-x86_64.tar.gz -C /tmp/agychat_tar .
rm -rf "$TAR_DIR"

# 5. Empaquetar Debian (.deb)
if command -v dpkg-deb >/dev/null 2>&1; then
    echo "📦 [3/5] Generando paquete Debian (.deb)..."
    DEB_DIR="/tmp/agychat_deb"
    rm -rf "$DEB_DIR"
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/usr/share/applications"
    mkdir -p "$DEB_DIR/usr/share/icons/hicolor/scalable/apps"
    mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"

    cp target/release/agychat "$DEB_DIR/usr/bin/"
    chmod 755 "$DEB_DIR/usr/bin/agychat"
    cp agychat.svg "$DEB_DIR/usr/share/icons/hicolor/scalable/apps/"
    [ -f agychat.png ] && cp agychat.png "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/"

    cat << 'EOF' > "$DEB_DIR/usr/share/applications/agychat.desktop"
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
EOF

    cat << EOF > "$DEB_DIR/DEBIAN/control"
Package: agychat
Version: ${VERSION}
Section: devel
Priority: optional
Architecture: amd64
Maintainer: Tiago Pujia <tiago@pujia.com>
Description: Antigravity Chat Manager — CLI & Slint Desktop GUI
 Herramienta para ramificar, duplicar, exportar e importar conversaciones
 en Google Antigravity entre proyectos o computadoras.
EOF

    dpkg-deb --root-owner-group --build "$DEB_DIR" "dist/agychat_${VERSION}_amd64.deb"
    rm -rf "$DEB_DIR"
fi

# 6. Empaquetar Fedora / RedHat (.rpm)
if command -v rpmbuild >/dev/null 2>&1; then
    echo "📦 [4/5] Generando paquete RPM (Fedora/RHEL)..."
    RPM_TOP="/tmp/rpmbuild"
    rm -rf "$RPM_TOP"
    mkdir -p "$RPM_TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

    cp target/release/agychat "$RPM_TOP/SOURCES/"
    cp agychat.svg "$RPM_TOP/SOURCES/"
    [ -f agychat.png ] && cp agychat.png "$RPM_TOP/SOURCES/"

    cat << 'EOF' > "$RPM_TOP/SOURCES/agychat.desktop"
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
EOF

    cat << EOF > "$RPM_TOP/SPECS/agychat.spec"
Name:           agychat
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Antigravity Chat Manager — CLI & Slint Desktop GUI
License:        MIT
URL:            https://github.com/Tiago-Pujia/agychat

%description
Antigravity Chat Manager — CLI & Slint Desktop GUI

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/scalable/apps
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps

install -m 755 %{_sourcedir}/agychat %{buildroot}/usr/bin/agychat
install -m 644 %{_sourcedir}/agychat.desktop %{buildroot}/usr/share/applications/agychat.desktop
install -m 644 %{_sourcedir}/agychat.svg %{buildroot}/usr/share/icons/hicolor/scalable/apps/agychat.svg
if [ -f %{_sourcedir}/agychat.png ]; then
    install -m 644 %{_sourcedir}/agychat.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/agychat.png
fi

%files
/usr/bin/agychat
/usr/share/applications/agychat.desktop
/usr/share/icons/hicolor/scalable/apps/agychat.svg
/usr/share/icons/hicolor/256x256/apps/agychat.png
EOF

    rpmbuild -bb --define "_topdir $RPM_TOP" "$RPM_TOP/SPECS/agychat.spec" >/dev/null 2>&1 || true
    if compgen -G "$RPM_TOP/RPMS/*/*.rpm" > /dev/null; then
        cp "$RPM_TOP"/RPMS/*/*.rpm dist/
    fi
    rm -rf "$RPM_TOP"
fi

# 7. Empaquetar Arch Linux (.pkg.tar.zst)
if command -v zstd >/dev/null 2>&1; then
    echo "📦 [5/5] Generando paquete Arch Linux (.pkg.tar.zst)..."
    ARCH_DIR="/tmp/agychat_arch"
    rm -rf "$ARCH_DIR"
    mkdir -p "$ARCH_DIR/usr/bin"
    mkdir -p "$ARCH_DIR/usr/share/applications"
    mkdir -p "$ARCH_DIR/usr/share/icons/hicolor/scalable/apps"
    mkdir -p "$ARCH_DIR/usr/share/icons/hicolor/256x256/apps"

    cp target/release/agychat "$ARCH_DIR/usr/bin/"
    chmod 755 "$ARCH_DIR/usr/bin/agychat"
    cp agychat.svg "$ARCH_DIR/usr/share/icons/hicolor/scalable/apps/"
    [ -f agychat.png ] && cp agychat.png "$ARCH_DIR/usr/share/icons/hicolor/256x256/apps/"

    cat << 'EOF' > "$ARCH_DIR/usr/share/applications/agychat.desktop"
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
EOF

    BUILD_DATE=$(date +%s)
    SIZE=$(du -sb "$ARCH_DIR" | awk '{print $1}')

    cat << EOF > "$ARCH_DIR/.PKGINFO"
pkgname = agychat
pkgbase = agychat
pkgver = ${VERSION}-1
pkgdesc = Antigravity Chat Manager — CLI & Slint Desktop GUI
url = https://github.com/Tiago-Pujia/agychat
builddate = ${BUILD_DATE}
packager = Tiago Pujia <tiago@pujia.com>
size = ${SIZE}
arch = x86_64
license = MIT
EOF

    tar --format=posix -C "$ARCH_DIR" -cf - .PKGINFO usr | zstd -c -T0 -19 > "dist/agychat-${VERSION}-1-x86_64.pkg.tar.zst"
    rm -rf "$ARCH_DIR"
fi

echo ""
echo "🎉 ¡Empaquetado completo en la carpeta dist/!"
ls -lh dist/
