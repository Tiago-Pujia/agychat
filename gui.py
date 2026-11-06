#!/usr/bin/env python3
"""
agychat GUI
Interfaz gráfica de escritorio moderna desarrollada en PyQt6 para gestionar,
duplicar, exportar e importar conversaciones de Google Antigravity.
"""

import json
import os
import re
import sys
from datetime import datetime

from PyQt6.QtCore import Qt, QUrl
from PyQt6.QtGui import QAction, QColor, QFont, QIcon, QKeySequence
from PyQt6.QtWidgets import (
    QAbstractItemView,
    QApplication,
    QCheckBox,
    QDialog,
    QFileDialog,
    QFrame,
    QHBoxLayout,
    QHeaderView,
    QLabel,
    QLineEdit,
    QMainWindow,
    QMenu,
    QMessageBox,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from duplicate_chat import (
    CONVERSATIONS_DIR,
    SUMMARIES_DB,
    delete_conversation,
    duplicate_conversation,
    export_conversation,
    find_conversation,
    get_live_ls_info,
    import_conversation,
)

# Estilo QSS Dark Mode Moderno
DARK_STYLE = """
QMainWindow, QDialog {
    background-color: #111418;
    color: #e2e8f0;
}

QWidget {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    font-size: 13px;
    color: #cbd5e1;
}

/* Campos de texto */
QLineEdit {
    background-color: #1e2229;
    border: 1px solid #2d333f;
    border-radius: 8px;
    padding: 7px 12px;
    color: #f8fafc;
    selection-background-color: #2563eb;
}
QLineEdit:focus {
    border: 1px solid #3b82f6;
    background-color: #1e2229;
}
QLineEdit:disabled {
    background-color: #181b20;
    color: #64748b;
    border: 1px solid #22262e;
}

/* Botones */
QPushButton {
    background-color: #1e2229;
    border: 1px solid #2d333f;
    border-radius: 8px;
    padding: 7px 16px;
    font-weight: 500;
    color: #f1f5f9;
}
QPushButton:hover {
    background-color: #282d37;
    border: 1px solid #3b82f6;
}
QPushButton:pressed {
    background-color: #181b20;
}
QPushButton:disabled {
    background-color: #16181d;
    border: 1px solid #1f232b;
    color: #475569;
}

/* Botón primario */
QPushButton#primaryBtn {
    background-color: #2563eb;
    border: 1px solid #3b82f6;
    color: #ffffff;
    font-weight: 600;
}
QPushButton#primaryBtn:hover {
    background-color: #1d4ed8;
}
QPushButton#primaryBtn:pressed {
    background-color: #1e40af;
}

/* Botón éxito */
QPushButton#successBtn {
    background-color: #059669;
    border: 1px solid #10b981;
    color: #ffffff;
    font-weight: 600;
}
QPushButton#successBtn:hover {
    background-color: #047857;
}

/* Botón peligro / eliminar */
QPushButton#dangerBtn {
    background-color: #1e2229;
    border: 1px solid #7f1d1d;
    color: #f87171;
    font-weight: 500;
}
QPushButton#dangerBtn:hover {
    background-color: #991b1b;
    border: 1px solid #ef4444;
    color: #ffffff;
}
QPushButton#dangerBtn:disabled {
    background-color: #16181d;
    border: 1px solid #1f232b;
    color: #475569;
}

/* Tabla */
QTableWidget {
    background-color: #15181e;
    border: 1px solid #22262e;
    border-radius: 10px;
    gridline-color: transparent;
    outline: none;
    selection-background-color: #1e3a8a;
    selection-color: #ffffff;
}
QTableWidget::item {
    padding: 8px 12px;
    border-bottom: 1px solid #1c2028;
}
QTableWidget::item:selected {
    background-color: rgba(37, 99, 235, 0.25);
    color: #60a5fa;
    border-left: 3px solid #3b82f6;
}

QHeaderView::section {
    background-color: #111418;
    color: #94a3b8;
    padding: 8px 12px;
    border: none;
    border-bottom: 1px solid #22262e;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
}

/* Checkbox */
QCheckBox {
    spacing: 8px;
    color: #cbd5e1;
}
QCheckBox::indicator {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1px solid #475569;
    background-color: #1e2229;
}
QCheckBox::indicator:checked {
    background-color: #2563eb;
    border: 1px solid #3b82f6;
}

/* Scrollbars */
QScrollBar:vertical {
    border: none;
    background: #111418;
    width: 8px;
    margin: 0px;
}
QScrollBar::handle:vertical {
    background: #334155;
    min-height: 20px;
    border-radius: 4px;
}
QScrollBar::handle:vertical:hover {
    background: #475569;
}
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
    height: 0px;
}

/* Menú contextual */
QMenu {
    background-color: #1e2229;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 4px;
}
QMenu::item {
    padding: 6px 20px;
    border-radius: 4px;
    color: #e2e8f0;
}
QMenu::item:selected {
    background-color: #2563eb;
    color: #ffffff;
}
"""


def format_relative_time(dt_str):
    """Convierte timestamp UTC a formato amigable."""
    if not dt_str:
        return "Desconocido"
    try:
        clean_str = dt_str.split("+")[0].split(".")[0]
        dt = datetime.strptime(clean_str, "%Y-%m-%d %H:%M:%S")
        now = datetime.utcnow()
        diff = now - dt

        if diff.days == 0:
            if diff.seconds < 60:
                return "Hace unos instantes"
            elif diff.seconds < 3600:
                mins = diff.seconds // 60
                return f"Hace {mins} min"
            else:
                hours = diff.seconds // 3600
                return f"Hoy {dt.strftime('%H:%M')}"
        elif diff.days == 1:
            return f"Ayer {dt.strftime('%H:%M')}"
        elif diff.days < 7:
            return f"Hace {diff.days} días"
        else:
            return dt.strftime("%d/%m/%Y")
    except Exception:
        return dt_str[:16]


class DuplicateModal(QDialog):
    """Modal emergente para ingresar el nuevo nombre de la copia."""

    def __init__(self, orig_title, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Duplicar Conversación")
        self.setFixedWidth(460)
        self.setModal(True)

        layout = QVBoxLayout(self)
        layout.setSpacing(16)
        layout.setContentsMargins(24, 24, 24, 24)

        # Encabezado
        title_lbl = QLabel("📋 Duplicar Conversación")
        title_lbl.setStyleSheet("font-size: 16px; font-weight: bold; color: #f8fafc;")
        layout.addWidget(title_lbl)

        # Campo chat original
        layout.addWidget(QLabel("Chat Original:"))
        self.orig_edit = QLineEdit(orig_title)
        self.orig_edit.setReadOnly(True)
        self.orig_edit.setEnabled(False)
        layout.addWidget(self.orig_edit)

        # Campo nuevo título
        layout.addWidget(QLabel("Nuevo Título (-t / --title):"))
        self.new_edit = QLineEdit(f"{orig_title} (Copia)")
        self.new_edit.selectAll()
        layout.addWidget(self.new_edit)

        sub_info = QLabel("El chat duplicado aparecerá de inmediato en la barra lateral de Antigravity.")
        sub_info.setStyleSheet("color: #64748b; font-size: 11px;")
        layout.addWidget(sub_info)

        # Botones de acción
        btn_layout = QHBoxLayout()
        btn_layout.addStretch()

        self.btn_cancel = QPushButton("Cancelar")
        self.btn_cancel.clicked.connect(self.reject)
        btn_layout.addWidget(self.btn_cancel)

        self.btn_confirm = QPushButton("Duplicar en Caliente")
        self.btn_confirm.setObjectName("primaryBtn")
        self.btn_confirm.clicked.connect(self.accept)
        btn_layout.addWidget(self.btn_confirm)

        layout.addLayout(btn_layout)

    def get_new_title(self):
        return self.new_edit.text().strip()


class ImportModal(QDialog):
    """Modal emergente completo para importar archivos .agychat con opciones avanzadas."""

    def __init__(self, initial_file="", parent=None):
        super().__init__(parent)
        self.setWindowTitle("Importar Chat")
        self.setFixedWidth(540)
        self.setModal(True)

        layout = QVBoxLayout(self)
        layout.setSpacing(14)
        layout.setContentsMargins(24, 24, 24, 24)

        # Título
        title_lbl = QLabel("📥 Importar Conversación (.agychat)")
        title_lbl.setStyleSheet("font-size: 16px; font-weight: bold; color: #f8fafc;")
        layout.addWidget(title_lbl)

        # Selector de archivo
        layout.addWidget(QLabel("Archivo .agychat a importar:"))
        file_layout = QHBoxLayout()
        self.file_edit = QLineEdit(initial_file)
        self.file_edit.setPlaceholderText("Selecciona o arrastra un archivo .agychat...")
        file_layout.addWidget(self.file_edit)

        self.btn_browse = QPushButton("Explorar...")
        self.btn_browse.clicked.connect(self.browse_file)
        file_layout.addWidget(self.btn_browse)
        layout.addLayout(file_layout)

        # Nuevo título opcional
        layout.addWidget(QLabel("Nuevo Título (Opcional, -t):"))
        self.title_edit = QLineEdit()
        self.title_edit.setPlaceholderText("Dejar vacío para usar el título original del archivo...")
        layout.addWidget(self.title_edit)

        # Workspace local opcional
        layout.addWidget(QLabel("Vincular a Workspace local (Opcional, -w):"))
        ws_layout = QHBoxLayout()
        self.ws_edit = QLineEdit()
        self.ws_edit.setPlaceholderText("Detectar automáticamente según el proyecto...")
        ws_layout.addWidget(self.ws_edit)

        self.btn_browse_ws = QPushButton("Buscar...")
        self.btn_browse_ws.clicked.connect(self.browse_workspace)
        ws_layout.addWidget(self.btn_browse_ws)
        layout.addLayout(ws_layout)

        # Checkboxes
        self.chk_new_id = QCheckBox("Generar nuevo identificador UUID único (--new-id)")
        self.chk_new_id.setChecked(True)
        layout.addWidget(self.chk_new_id)

        self.chk_overwrite = QCheckBox("Sobrescribir si el UUID ya existe en este sistema (--overwrite)")
        layout.addWidget(self.chk_overwrite)

        # Botones
        btn_layout = QHBoxLayout()
        btn_layout.addStretch()

        btn_cancel = QPushButton("Cancelar")
        btn_cancel.clicked.connect(self.reject)
        btn_layout.addWidget(btn_cancel)

        btn_import = QPushButton("Iniciar Importación")
        btn_import.setObjectName("successBtn")
        btn_import.clicked.connect(self.validate_and_accept)
        btn_layout.addWidget(btn_import)

        layout.addLayout(btn_layout)

    def browse_file(self):
        fpath, _ = QFileDialog.getOpenFileName(
            self,
            "Seleccionar chat de Antigravity",
            "",
            "Archivos de Antigravity (*.agychat *.tar.gz);;Todos los archivos (*)",
        )
        if fpath:
            self.file_edit.setText(fpath)

    def browse_workspace(self):
        dir_path = QFileDialog.getExistingDirectory(self, "Seleccionar carpeta del Workspace/Proyecto")
        if dir_path:
            self.ws_edit.setText(dir_path)

    def validate_and_accept(self):
        path = self.file_edit.text().strip()
        if not path or not os.path.exists(path):
            QMessageBox.warning(self, "Archivo no encontrado", "Debes seleccionar un archivo .agychat válido existente.")
            return
        self.accept()

    def get_options(self):
        return {
            "archive_path": self.file_edit.text().strip(),
            "custom_title": self.title_edit.text().strip() or None,
            "target_workspace": self.ws_edit.text().strip() or None,
            "new_id": self.chk_new_id.isChecked(),
            "overwrite": self.chk_overwrite.isChecked(),
        }


class MainWindow(QMainWindow):
    """Ventana principal de la aplicación agychat."""

    def __init__(self):
        super().__init__()
        self.setWindowTitle("agychat — Antigravity Chat Manager")
        self.resize(960, 600)
        self.setMinimumSize(780, 480)
        self.setAcceptDrops(True)

        # Cargar icono de la aplicación si existe
        icon_path = os.path.join(os.path.dirname(__file__), "agychat.svg")
        if os.path.exists(icon_path):
            self.setWindowIcon(QIcon(icon_path))

        self.conversations = []
        self.selected_cid = None
        self.selected_title = None

        self.setup_ui()
        self.refresh_data()

    def setup_ui(self):
        central = QWidget()
        self.setCentralWidget(central)
        main_layout = QVBoxLayout(central)
        main_layout.setContentsMargins(18, 16, 18, 16)
        main_layout.setSpacing(14)

        # 1. Barra de herramientas superior
        top_bar = QHBoxLayout()
        top_bar.setSpacing(10)

        self.search_edit = QLineEdit()
        self.search_edit.setPlaceholderText("🔍 Filtrar por título, proyecto o UUID...")
        self.search_edit.textChanged.connect(self.filter_table)
        top_bar.addWidget(self.search_edit, stretch=1)

        self.btn_refresh = QPushButton("🔄 Refrescar (-l)")
        self.btn_refresh.clicked.connect(self.refresh_data)
        top_bar.addWidget(self.btn_refresh)

        self.btn_import_top = QPushButton("📥 Importar (.agychat)")
        self.btn_import_top.setObjectName("primaryBtn")
        self.btn_import_top.clicked.connect(self.open_import_dialog)
        top_bar.addWidget(self.btn_import_top)

        main_layout.addLayout(top_bar)

        # 2. Tabla de conversaciones
        self.table = QTableWidget()
        self.table.setColumnCount(5)
        self.table.setHorizontalHeaderLabels([
            "Conversación",
            "Proyecto / Workspace",
            "Pasos",
            "Modificación",
            "UUID",
        ])
        self.table.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        self.table.setSelectionMode(QAbstractItemView.SelectionMode.SingleSelection)
        self.table.setEditTriggers(QAbstractItemView.EditTrigger.NoEditTriggers)
        self.table.setShowGrid(False)
        self.table.verticalHeader().setVisible(False)
        self.table.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.table.customContextMenuRequested.connect(self.show_context_menu)
        self.table.itemSelectionChanged.connect(self.on_selection_changed)
        self.table.itemDoubleClicked.connect(self.on_double_click_row)

        header = self.table.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        header.setSectionResizeMode(1, QHeaderView.ResizeMode.ResizeToContents)
        header.setSectionResizeMode(2, QHeaderView.ResizeMode.ResizeToContents)
        header.setSectionResizeMode(3, QHeaderView.ResizeMode.ResizeToContents)
        header.setSectionResizeMode(4, QHeaderView.ResizeMode.ResizeToContents)

        main_layout.addWidget(self.table, stretch=1)

        # 3. Barra de acciones inferior
        action_bar = QHBoxLayout()
        action_bar.setSpacing(12)

        self.lbl_selection = QLabel("Selecciona una conversación para duplicar o exportar.")
        self.lbl_selection.setStyleSheet("color: #64748b; font-size: 12px;")
        action_bar.addWidget(self.lbl_selection, stretch=1)

        self.btn_dup = QPushButton("📋 Duplicar (-d)")
        self.btn_dup.setEnabled(False)
        self.btn_dup.clicked.connect(self.action_duplicate)
        action_bar.addWidget(self.btn_dup)

        self.btn_exp = QPushButton("📦 Exportar (-e)")
        self.btn_exp.setEnabled(False)
        self.btn_exp.clicked.connect(self.action_export)
        action_bar.addWidget(self.btn_exp)

        self.btn_del = QPushButton("🗑️ Eliminar (-D)")
        self.btn_del.setObjectName("dangerBtn")
        self.btn_del.setEnabled(False)
        self.btn_del.clicked.connect(self.action_delete)
        action_bar.addWidget(self.btn_del)

        main_layout.addLayout(action_bar)

        # 4. Barra de estado inferior
        self.status_frame = QFrame()
        self.status_frame.setStyleSheet(
            "background-color: #161a22; border-radius: 8px; padding: 4px 10px; border: 1px solid #232731;"
        )
        status_layout = QHBoxLayout(self.status_frame)
        status_layout.setContentsMargins(6, 4, 6, 4)

        self.lbl_ls_status = QLabel()
        self.lbl_ls_status.setStyleSheet("font-size: 11px;")
        status_layout.addWidget(self.lbl_ls_status)

        status_layout.addStretch()

        self.lbl_stats = QLabel()
        self.lbl_stats.setStyleSheet("color: #94a3b8; font-size: 11px;")
        status_layout.addWidget(self.lbl_stats)

        main_layout.addWidget(self.status_frame)

    def refresh_data(self):
        """Lee la base de datos de SQLite y actualiza la tabla."""
        import sqlite3

        if not os.path.exists(SUMMARIES_DB):
            self.lbl_ls_status.setText("❌ No se encontró la base de datos de Antigravity.")
            return

        con = sqlite3.connect(SUMMARIES_DB, timeout=10.0)
        con.row_factory = sqlite3.Row
        rows = con.execute(
            "SELECT conversation_id, title, step_count, last_modified_time, workspace_uris, project_id "
            "FROM conversation_summaries ORDER BY last_modified_time DESC"
        ).fetchall()
        con.close()

        self.conversations = [dict(r) for r in rows]
        self.populate_table(self.conversations)

        # Actualizar estado de Antigravity
        port, csrf = get_live_ls_info()
        if port and csrf:
            self.lbl_ls_status.setText(f"🟢 Antigravity conectado (puerto {port})")
            self.lbl_ls_status.setStyleSheet("color: #10b981; font-size: 11px; font-weight: 500;")
        else:
            self.lbl_ls_status.setText("🟠 Antigravity no detectado en ejecución (los cambios se verán al abrirlo)")
            self.lbl_ls_status.setStyleSheet("color: #f59e0b; font-size: 11px;")

        self.lbl_stats.setText(f"{len(self.conversations)} conversaciones disponibles")

    def populate_table(self, convs):
        self.table.setRowCount(0)
        for i, row in enumerate(convs):
            self.table.insertRow(i)
            title = row.get("title") or "(Sin título)"
            cid = row.get("conversation_id", "")
            steps = str(row.get("step_count", 0))
            mod_time = format_relative_time(row.get("last_modified_time", ""))

            # Workspace / Proyecto
            ws_raw = row.get("workspace_uris", "")
            ws_display = "(Sin workspace)"
            if ws_raw:
                try:
                    uris = json.loads(ws_raw) if ws_raw.startswith("[") else [ws_raw]
                    if uris and uris[0]:
                        ws_display = os.path.basename(uris[0].rstrip("/"))
                except Exception:
                    ws_display = ws_raw

            item_title = QTableWidgetItem(title)
            item_title.setData(Qt.ItemDataRole.UserRole, cid)
            f = item_title.font()
            f.setBold(True)
            item_title.setFont(f)

            item_ws = QTableWidgetItem(ws_display)
            item_ws.setForeground(QColor("#94a3b8"))

            item_steps = QTableWidgetItem(steps)
            item_steps.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
            item_steps.setForeground(QColor("#60a5fa"))

            item_time = QTableWidgetItem(mod_time)
            item_time.setForeground(QColor("#64748b"))

            item_uuid = QTableWidgetItem(f"{cid[:8]}...")
            item_uuid.setForeground(QColor("#475569"))
            item_uuid.setToolTip(cid)

            self.table.setItem(i, 0, item_title)
            self.table.setItem(i, 1, item_ws)
            self.table.setItem(i, 2, item_steps)
            self.table.setItem(i, 3, item_time)
            self.table.setItem(i, 4, item_uuid)

        # Restaurar selección si aplica
        self.on_selection_changed()

    def filter_table(self, query):
        q = query.strip().lower()
        if not q:
            filtered = self.conversations
        else:
            filtered = [
                c
                for c in self.conversations
                if q in (c.get("title") or "").lower()
                or q in c.get("conversation_id", "").lower()
                or q in (c.get("workspace_uris") or "").lower()
            ]
        self.populate_table(filtered)

    def on_selection_changed(self):
        sel = self.table.selectedItems()
        if not sel:
            self.selected_cid = None
            self.selected_title = None
            self.btn_dup.setEnabled(False)
            self.btn_exp.setEnabled(False)
            self.btn_del.setEnabled(False)
            self.lbl_selection.setText("Selecciona una conversación para duplicar, exportar o eliminar.")
            return

        row = self.table.currentRow()
        item = self.table.item(row, 0)
        if item:
            self.selected_cid = item.data(Qt.ItemDataRole.UserRole)
            self.selected_title = item.text()
            self.btn_dup.setEnabled(True)
            self.btn_exp.setEnabled(True)
            self.btn_del.setEnabled(True)
            self.lbl_selection.setText(
                f"Seleccionado: <b style='color:#f8fafc;'>\"{self.selected_title}\"</b> "
                f"<span style='color:#64748b;'>({self.selected_cid[:8]}...)</span>"
            )

    def on_double_click_row(self, item):
        self.action_duplicate()

    def show_context_menu(self, pos):
        item = self.table.itemAt(pos)
        if not item:
            return
        menu = QMenu(self)

        act_dup = menu.addAction("📋 Duplicar conversación")
        act_exp = menu.addAction("📦 Exportar a .agychat")
        act_del = menu.addAction("🗑️ Eliminar conversación")
        menu.addSeparator()
        act_copy = menu.addAction("📋 Copiar UUID al portapapeles")

        action = menu.exec(self.table.viewport().mapToGlobal(pos))
        if action == act_dup:
            self.action_duplicate()
        elif action == act_exp:
            self.action_export()
        elif action == act_del:
            self.action_delete()
        elif action == act_copy and self.selected_cid:
            QApplication.clipboard().setText(self.selected_cid)
            self.statusBar().showMessage("UUID copiado al portapapeles", 3000)

    def action_delete(self):
        if not self.selected_cid or not self.selected_title:
            return

        reply = QMessageBox.warning(
            self,
            "Eliminar Conversación",
            f"¿Estás seguro de que deseas eliminar permanentemente esta conversación?\n\n"
            f"• Título: \"{self.selected_title}\"\n"
            f"• ID: {self.selected_cid}\n\n"
            "Esta acción borrará el historial de la base de datos y descargará el chat de Antigravity.",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            deleted_title = self.selected_title
            success = delete_conversation(self.selected_cid, force=True)
            if success:
                self.refresh_data()
                self.statusBar().showMessage(f"Conversación \"{deleted_title}\" eliminada.", 4000)
            else:
                QMessageBox.critical(
                    self,
                    "Error al Eliminar",
                    "No se pudo eliminar la conversación.",
                )

    def action_duplicate(self):
        if not self.selected_cid or not self.selected_title:
            return

        modal = DuplicateModal(self.selected_title, self)
        if modal.exec() == QDialog.DialogCode.Accepted:
            new_title = modal.get_new_title()
            new_id = duplicate_conversation(self.selected_cid, new_title)
            if new_id:
                QMessageBox.information(
                    self,
                    "Duplicación Exitosa",
                    f"¡El chat ha sido duplicado con éxito!\n\nTítulo: \"{new_title}\"\n\n"
                    "Ya se encuentra visible en la barra lateral de Antigravity.",
                )
                self.refresh_data()
            else:
                QMessageBox.critical(
                    self,
                    "Error al Duplicar",
                    "No se pudo duplicar el chat. Verifica que Antigravity se encuentre en ejecución.",
                )

    def action_export(self):
        if not self.selected_cid or not self.selected_title:
            return

        clean_name = re.sub(r"[^\w\s-]", "", self.selected_title).strip()
        clean_name = re.sub(r"[-\s]+", "_", clean_name)
        suggested = f"{clean_name or 'antigravity_chat'}.agychat"

        fpath, _ = QFileDialog.getSaveFileName(
            self,
            "Exportar conversación a archivo portable",
            suggested,
            "Antigravity Chat (*.agychat);;Archivo Comprimido (*.tar.gz)",
        )
        if not fpath:
            return

        out = export_conversation(self.selected_cid, fpath)
        if out and os.path.exists(out):
            size_mb = os.path.getsize(out) / (1024 * 1024)
            QMessageBox.information(
                self,
                "Exportación Exitosa",
                f"La conversación fue empaquetada correctamente.\n\n"
                f"• Archivo: {out}\n"
                f"• Tamaño: {size_mb:.2f} MB\n\n"
                "Puedes transferirlo a otra computadora e importarlo con agychat.",
            )

    def open_import_dialog(self, prefill_file=""):
        modal = ImportModal(prefill_file, self)
        if modal.exec() == QDialog.DialogCode.Accepted:
            opts = modal.get_options()
            imported_id = import_conversation(
                opts["archive_path"],
                custom_title=opts["custom_title"],
                target_workspace=opts["target_workspace"],
                new_id=opts["new_id"],
                overwrite=opts["overwrite"],
            )
            if imported_id:
                QMessageBox.information(
                    self,
                    "Importación Exitosa",
                    "El chat fue importado y cargado en caliente en Antigravity con éxito.",
                )
                self.refresh_data()
            else:
                QMessageBox.critical(
                    self,
                    "Error al Importar",
                    "Ocurrió un error al procesar el archivo. Revisa los detalles en la consola.",
                )

    # Soporte Drag & Drop
    def dragEnterEvent(self, event):
        if event.mimeData().hasUrls():
            for url in event.mimeData().urls():
                fn = url.toLocalFile()
                if fn.endswith(".agychat") or fn.endswith(".tar.gz"):
                    event.acceptProposedAction()
                    return
        event.ignore()

    def dropEvent(self, event):
        for url in event.mimeData().urls():
            fn = url.toLocalFile()
            if fn.endswith(".agychat") or fn.endswith(".tar.gz"):
                event.acceptProposedAction()
                self.open_import_dialog(fn)
                break


def launch_gui():
    """Punto de entrada para ejecutar la interfaz gráfica."""
    app = QApplication(sys.argv)
    app.setApplicationName("agychat")
    app.setApplicationDisplayName("agychat - Antigravity Chat Manager")
    app.setDesktopFileName("agychat.desktop")

    # Aplicar icono global
    icon_path = os.path.join(os.path.dirname(__file__), "agychat.svg")
    if os.path.exists(icon_path):
        app.setWindowIcon(QIcon(icon_path))

    app.setStyleSheet(DARK_STYLE)

    window = MainWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    launch_gui()
