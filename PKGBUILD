# Maintainer: Tiago Pujia <tiago@pujia.com>
pkgname=agychat
pkgver=2.0.3
pkgrel=1
pkgdesc="Antigravity Chat Manager — CLI & Slint Desktop GUI"
arch=('x86_64')
url="https://github.com/Tiago-Pujia/agychat"
license=('MIT')
depends=('fontconfig' 'libx11' 'libxkbcommon' 'libxcb')
makedepends=('cargo' 'git')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/Tiago-Pujia/agychat/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
    cd "${pkgname}-${pkgver}"
    cargo build --release --locked
}

package() {
    cd "${pkgname}-${pkgver}"
    install -Dm755 "target/release/agychat" "${pkgdir}/usr/bin/agychat"
    install -Dm644 "agychat.svg" "${pkgdir}/usr/share/icons/hicolor/scalable/apps/agychat.svg"
    if [ -f "agychat.png" ]; then
        install -Dm644 "agychat.png" "${pkgdir}/usr/share/icons/hicolor/256x256/apps/agychat.png"
    fi
    install -Dm644 "install.sh" -t "${pkgdir}/usr/share/doc/agychat/"
    install -Dm644 "README.md" "${pkgdir}/usr/share/doc/agychat/README.md"

    mkdir -p "${pkgdir}/usr/share/applications"
    cat << EOF > "${pkgdir}/usr/share/applications/agychat.desktop"
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
}
