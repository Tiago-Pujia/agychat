#!/usr/bin/env python3
"""
Antigravity Duplicate Chat
Herramienta de línea de comandos para ramificar, duplicar, exportar e importar
conversaciones en Google Antigravity en tiempo real sin perder el contexto.
"""

import argparse
import datetime
import io
import json
import os
import re
import shutil
import sqlite3
import ssl
import sys
import tarfile
import tempfile
import urllib.request
import uuid

BASE_DIR = os.path.expanduser("~/.gemini/antigravity")
SUMMARIES_DB = os.path.join(BASE_DIR, "conversation_summaries.db")
CONVERSATIONS_DIR = os.path.join(BASE_DIR, "conversations")
ANNOTATIONS_DIR = os.path.join(BASE_DIR, "annotations")
BRAIN_DIR = os.path.join(BASE_DIR, "brain")
LS_LOG = os.path.expanduser("~/.config/Antigravity/logs/language_server.log")


def get_live_ls_info():
    """Detecta si language_server está corriendo y obtiene su puerto HTTPS y CSRF token."""
    csrf_token = None
    try:
        for pid in os.listdir("/proc"):
            if not pid.isdigit():
                continue
            cmdline_path = os.path.join("/proc", pid, "cmdline")
            if os.path.exists(cmdline_path):
                with open(cmdline_path, "rb") as f:
                    cmdline = f.read().decode("utf-8", errors="ignore")
                if "language_server" in cmdline and "--csrf_token" in cmdline:
                    m = re.search(
                        r"--csrf_token\x00([a-f0-9-]+)", cmdline
                    ) or re.search(r"--csrf_token\s+([a-f0-9-]+)", cmdline)
                    if m:
                        csrf_token = m.group(1)
                    break
    except Exception:
        pass

    if not csrf_token:
        return None, None

    port = None
    if os.path.exists(LS_LOG):
        with open(LS_LOG, "r", encoding="utf-8", errors="ignore") as f:
            lines = f.readlines()
        for line in reversed(lines):
            m = re.search(r"listening on random port at (\d+) for HTTPS", line)
            if m:
                port = int(m.group(1))
                break

    return port, csrf_token


def fork_via_api(source_id, new_title, port, csrf_token, step_count=71):
    """Invoca los endpoints Connect/gRPC en vivo de Antigravity para bifurcar un chat."""
    ctx = ssl._create_unverified_context()
    headers = {
        "Content-Type": "application/json",
        "X-Codeium-Csrf-Token": csrf_token,
    }

    # 1. Fork conversation (clona historial y contexto)
    fork_url = f"https://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/ForkConversation"
    fork_data = json.dumps(
        {
            "source_cascade_id": source_id,
            "fork_at_step_index": max(0, step_count - 1),
        }
    ).encode("utf-8")

    req = urllib.request.Request(fork_url, data=fork_data, headers=headers)
    res = urllib.request.urlopen(req, timeout=10, context=ctx)
    res_json = json.loads(res.read().decode("utf-8"))
    new_cascade_id = res_json.get("newCascadeId")

    if not new_cascade_id:
        raise Exception(f"No se recibió newCascadeId en la respuesta: {res_json}")

    # 2. Update conversation title (asigna el nombre personalizado)
    update_annotations_via_api(new_cascade_id, new_title, port, csrf_token)

    return new_cascade_id


def load_trajectory_via_api(cascade_id, port, csrf_token):
    """Solicita a language_server cargar una trayectoria de conversación en memoria."""
    ctx = ssl._create_unverified_context()
    headers = {
        "Content-Type": "application/json",
        "X-Codeium-Csrf-Token": csrf_token,
    }
    url = f"https://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/LoadTrajectory"
    data = json.dumps({"cascade_id": cascade_id}).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers)
    try:
        urllib.request.urlopen(req, timeout=5, context=ctx)
        return True
    except Exception:
        return False


def update_annotations_via_api(cascade_id, title, port, csrf_token):
    """Actualiza anotaciones de título y refresca la interfaz gráfica de Antigravity."""
    ctx = ssl._create_unverified_context()
    headers = {
        "Content-Type": "application/json",
        "X-Codeium-Csrf-Token": csrf_token,
    }
    annot_url = f"https://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/UpdateConversationAnnotations"
    annot_data = json.dumps(
        {
            "cascade_id": cascade_id,
            "cascade_ids": [cascade_id],
            "annotations": {"title": title},
            "merge_annotations": True,
        }
    ).encode("utf-8")
    req_annot = urllib.request.Request(annot_url, data=annot_data, headers=headers)
    try:
        urllib.request.urlopen(req_annot, timeout=10, context=ctx)
        return True
    except Exception:
        return False


def find_conversation(source_id_or_title):
    """Busca una conversación en la base de datos de resúmenes por ID o título."""
    if not os.path.exists(SUMMARIES_DB):
        return None
    con = sqlite3.connect(SUMMARIES_DB, timeout=30.0)
    con.row_factory = sqlite3.Row
    row = con.execute(
        "SELECT * FROM conversation_summaries WHERE conversation_id = ? OR title LIKE ? ORDER BY last_modified_time DESC LIMIT 1",
        (source_id_or_title, f"%{source_id_or_title}%"),
    ).fetchone()
    con.close()
    return dict(row) if row else None


def list_conversations():
    """Muestra la lista de chats registrados en la base de datos de Antigravity."""
    if not os.path.exists(SUMMARIES_DB):
        print("❌ No se encontró la base de datos de chats de Antigravity.")
        return
    con = sqlite3.connect(SUMMARIES_DB)
    rows = con.execute(
        "SELECT title, conversation_id FROM conversation_summaries ORDER BY last_modified_time DESC"
    ).fetchall()
    con.close()

    print("\n📋 Chats disponibles en Antigravity:")
    for title, cid in rows:
        display_title = title if title.strip() else "(Sin título)"
        print(f"  • \"{display_title}\"  \033[90m[{cid}]\033[0m")
    print()


def duplicate_conversation(source_id_or_title, custom_new_title=None):
    """Duplica una conversación local en caliente a través de la API de Antigravity."""
    if not os.path.exists(SUMMARIES_DB):
        print("❌ Error: No se encontró la base de datos de Antigravity.")
        return None

    row = find_conversation(source_id_or_title)
    if not row:
        print(f"\n❌ Error: No se encontró ningún chat que coincida con '{source_id_or_title}'.")
        list_conversations()
        return None

    source_id = row["conversation_id"]
    source_title = row["title"]
    step_count = row["step_count"]

    new_title = custom_new_title or f"{source_title} (Copia)"

    print(f"\n🚀 Duplicando chat en Antigravity:")
    print(f"   • Original: \"{source_title}\"")
    print(f"   • Nueva:    \"{new_title}\"")

    port, csrf_token = get_live_ls_info()
    if not (port and csrf_token):
        print("\n❌ Error: Antigravity no parece estar en ejecución.")
        print("   Abre la aplicación Antigravity antes de ejecutar la duplicación.\n")
        return None

    try:
        new_id = fork_via_api(source_id, new_title, port, csrf_token, step_count)
        print(f"\n✅ ¡Listo! El chat ha sido duplicado con éxito.")
        print(f"   Ya aparece visible en tu barra lateral como \"{new_title}\".\n")
        return new_id
    except Exception as e:
        print(f"\n❌ Error al duplicar en vivo: {e}\n")
        return None


def export_conversation(source_id_or_title, output_path=None):
    """Empaqueta una conversación completa en un archivo comprimido .agychat (tar.gz)."""
    if not os.path.exists(SUMMARIES_DB):
        print("❌ Error: No se encontró la base de datos de Antigravity.")
        return None

    row = find_conversation(source_id_or_title)
    if not row:
        print(f"\n❌ Error: No se encontró ningún chat que coincida con '{source_id_or_title}'.")
        list_conversations()
        return None

    cid = row["conversation_id"]
    title = row["title"] or "Chat_Sin_Titulo"
    step_count = row["step_count"]

    db_path = os.path.join(CONVERSATIONS_DIR, f"{cid}.db")
    if not os.path.exists(db_path):
        print(f"❌ Error: No se encontró el archivo de base de datos de la conversación: {db_path}")
        return None

    if not output_path:
        clean_title = re.sub(r"[^\w\s-]", "", title).strip()
        clean_title = re.sub(r"[-\s]+", "_", clean_title)
        output_path = f"{clean_title or 'antigravity_chat'}.agychat"

    output_path = os.path.abspath(output_path)
    output_dir = os.path.dirname(output_path)
    if output_dir and not os.path.exists(output_dir):
        os.makedirs(output_dir, exist_ok=True)

    print(f"\n📦 Exportando chat de Antigravity:")
    print(f"   • Título:  \"{title}\"")
    print(f"   • ID:      {cid}")
    print(f"   • Pasos:   {step_count}")
    print(f"   • Destino: {output_path}")

    raw_summary = row.get("raw_summary")
    summary_dict = {k: v for k, v in row.items() if k != "raw_summary"}

    orig_workspaces = []
    if row.get("workspace_uris"):
        try:
            orig_workspaces = json.loads(row["workspace_uris"])
        except Exception:
            orig_workspaces = [row["workspace_uris"]]

    manifest = {
        "format": "antigravity-chat-archive",
        "version": 1,
        "conversation_id": cid,
        "title": title,
        "step_count": step_count,
        "workspace_uris": orig_workspaces,
        "project_id": row.get("project_id", ""),
        "source_home": os.path.expanduser("~"),
        "source_user": os.environ.get("USER", ""),
        "last_modified_time": row.get("last_modified_time", ""),
        "exported_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    }

    with tarfile.open(output_path, "w:gz") as tar:
        # 1. manifest.json
        manifest_bytes = json.dumps(manifest, indent=2, ensure_ascii=False).encode("utf-8")
        ti_manifest = tarfile.TarInfo(name="manifest.json")
        ti_manifest.size = len(manifest_bytes)
        ti_manifest.mtime = int(datetime.datetime.now().timestamp())
        tar.addfile(ti_manifest, io.BytesIO(manifest_bytes))

        # 2. summary.json
        summary_bytes = json.dumps(summary_dict, indent=2, ensure_ascii=False).encode("utf-8")
        ti_summary = tarfile.TarInfo(name="summary.json")
        ti_summary.size = len(summary_bytes)
        ti_summary.mtime = int(datetime.datetime.now().timestamp())
        tar.addfile(ti_summary, io.BytesIO(summary_bytes))

        # 3. raw_summary.bin (si existe)
        if raw_summary:
            ti_raw = tarfile.TarInfo(name="raw_summary.bin")
            ti_raw.size = len(raw_summary)
            ti_raw.mtime = int(datetime.datetime.now().timestamp())
            tar.addfile(ti_raw, io.BytesIO(raw_summary))

        # 4. conversation.db
        tar.add(db_path, arcname="conversation.db")

        # 5. annotation.pbtxt (si existe)
        pbtxt_path = os.path.join(ANNOTATIONS_DIR, f"{cid}.pbtxt")
        if os.path.exists(pbtxt_path):
            tar.add(pbtxt_path, arcname="annotation.pbtxt")

        # 6. brain/ (si existe)
        brain_path = os.path.join(BRAIN_DIR, cid)
        if os.path.exists(brain_path):
            tar.add(brain_path, arcname="brain")

    size_mb = os.path.getsize(output_path) / (1024 * 1024)
    print(f"\n✅ ¡Exportación completada con éxito! ({size_mb:.2f} MB)")
    print(f"   Para importarlo en otra máquina ejecuta:")
    print(f"   agychat -i \"{output_path}\"\n")
    return output_path


def import_conversation(
    archive_path,
    custom_title=None,
    target_workspace=None,
    new_id=False,
    overwrite=False,
):
    """Importa una conversación desde un archivo .agychat adaptando rutas y cargando en caliente."""
    if not os.path.exists(archive_path):
        print(f"❌ Error: El archivo '{archive_path}' no existe.")
        return None

    if not os.path.exists(BASE_DIR):
        print("❌ Error: No se encontró el directorio de Antigravity (~/.gemini/antigravity).")
        return None

    os.makedirs(CONVERSATIONS_DIR, exist_ok=True)
    os.makedirs(ANNOTATIONS_DIR, exist_ok=True)
    os.makedirs(BRAIN_DIR, exist_ok=True)

    with tempfile.TemporaryDirectory() as temp_dir:
        try:
            with tarfile.open(archive_path, "r:gz") as tar:
                if hasattr(tarfile, "data_filter"):
                    tar.extractall(temp_dir, filter="data")
                else:
                    tar.extractall(temp_dir)
        except Exception as e:
            print(f"❌ Error al abrir o descomprimir el archivo: {e}")
            return None

        manifest_file = os.path.join(temp_dir, "manifest.json")
        summary_file = os.path.join(temp_dir, "summary.json")
        db_file = os.path.join(temp_dir, "conversation.db")

        if not (os.path.exists(manifest_file) and os.path.exists(db_file)):
            print("❌ Error: El archivo no tiene la estructura válida de un chat de Antigravity.")
            return None

        with open(manifest_file, "r", encoding="utf-8") as f:
            manifest = json.load(f)

        summary_dict = {}
        if os.path.exists(summary_file):
            with open(summary_file, "r", encoding="utf-8") as f:
                summary_dict = json.load(f)

        raw_summary_bytes = None
        raw_summary_file = os.path.join(temp_dir, "raw_summary.bin")
        if os.path.exists(raw_summary_file):
            with open(raw_summary_file, "rb") as f:
                raw_summary_bytes = f.read()

        orig_cid = manifest.get("conversation_id")
        orig_title = manifest.get("title", "Chat Importado")
        final_title = custom_title or orig_title

        con = sqlite3.connect(SUMMARIES_DB, timeout=30.0)
        existing = con.execute(
            "SELECT conversation_id, title FROM conversation_summaries WHERE conversation_id = ?",
            (orig_cid,),
        ).fetchone()
        con.close()

        if new_id:
            target_cid = str(uuid.uuid4())
            print(f"ℹ️  Generando un nuevo identificador único: {target_cid}")
        elif existing:
            if overwrite:
                target_cid = orig_cid
                print(f"⚠️  El chat con ID '{orig_cid}' ya existe. Sobrescribiendo según lo indicado (--overwrite)...")
            else:
                target_cid = str(uuid.uuid4())
                print(f"⚠️  El chat '{existing[1]}' ya existe en este sistema.")
                print(f"   Asignando automáticamente un nuevo ID único ({target_cid}) para evitar sobreescritura.")
        else:
            target_cid = orig_cid

        # Resolver workspace en máquina destino
        orig_workspaces = manifest.get("workspace_uris", [])
        remapped_workspaces = []

        if target_workspace:
            abs_ws = os.path.abspath(os.path.expanduser(target_workspace))
            remapped_workspaces = [f"file://{abs_ws}"]
        else:
            source_home = manifest.get("source_home", "")
            target_home = os.path.expanduser("~")
            if source_home and target_home != source_home:
                for ws in orig_workspaces:
                    remapped_workspaces.append(ws.replace(f"file://{source_home}", f"file://{target_home}"))
            else:
                remapped_workspaces = list(orig_workspaces)

            # Si la ruta no existe en el equipo, verificar si estamos en un repo o directorio de proyecto
            if remapped_workspaces:
                check_path = remapped_workspaces[0].replace("file://", "")
                if not os.path.exists(check_path):
                    cwd = os.getcwd()
                    if os.path.exists(os.path.join(cwd, ".git")):
                        print(f"💡 Ruta de workspace original ({check_path}) no encontrada en este equipo.")
                        print(f"   Asociando automáticamente al repositorio actual: {cwd}")
                        remapped_workspaces = [f"file://{cwd}"]

        # Resolver project_id en destino
        target_project_id = ""
        if remapped_workspaces:
            # 1. Buscar primero en los proyectos registrados en Antigravity (~/.gemini/config/projects)
            projects_dir = os.path.expanduser("~/.gemini/config/projects")
            if os.path.exists(projects_dir):
                norm_uri = remapped_workspaces[0].rstrip("/").lower()
                for fname in os.listdir(projects_dir):
                    if not fname.endswith(".json"):
                        continue
                    try:
                        with open(os.path.join(projects_dir, fname), "r", encoding="utf-8") as pf:
                            pdata = json.load(pf)
                        resources = pdata.get("projectResources", {}).get("resources", [])
                        for r in resources:
                            uri = r.get("folderUri") or r.get("gitFolder", {}).get("folderUri", "")
                            if uri and uri.rstrip("/").lower() == norm_uri:
                                target_project_id = pdata.get("id", "")
                                break
                        if target_project_id:
                            break
                    except Exception:
                        pass

            # 2. Si no se encontró, buscar en los chats existentes en conversation_summaries.db
            if not target_project_id:
                con = sqlite3.connect(SUMMARIES_DB, timeout=30.0)
                p_row = con.execute(
                    "SELECT project_id FROM conversation_summaries WHERE workspace_uris LIKE ? AND project_id != '' LIMIT 1",
                    (f"%{remapped_workspaces[0]}%",),
                ).fetchone()
                con.close()
                if p_row:
                    target_project_id = p_row[0]

        if not target_project_id:
            target_project_id = manifest.get("project_id") or str(uuid.uuid4())

        orig_project_id = manifest.get("project_id", "")

        print(f"\n📥 Importando chat en Antigravity:")
        print(f"   • Título:    \"{final_title}\"")
        print(f"   • ID:        {target_cid}")
        print(f"   • Workspace: {remapped_workspaces[0] if remapped_workspaces else '(ninguno)'}")
        print(f"   • Project:   {target_project_id}")

        # 1. Copiar base de datos de la conversación y actualizar cascade_id y metadata blob
        dest_db = os.path.join(CONVERSATIONS_DIR, f"{target_cid}.db")
        shutil.copyfile(db_file, dest_db)
        conv_con = sqlite3.connect(dest_db)
        conv_con.execute("UPDATE trajectory_meta SET cascade_id = ?", (target_cid,))
        
        # Parchear project_id y cascade_id en trajectory_metadata_blob si cambiaron
        try:
            cur = conv_con.cursor()
            cur.execute("SELECT data FROM trajectory_metadata_blob WHERE id = 'main'")
            b_row = cur.fetchone()
            if b_row and b_row[0]:
                blob = b_row[0]
                if orig_project_id and target_project_id and orig_project_id != target_project_id:
                    blob = blob.replace(orig_project_id.encode("utf-8"), target_project_id.encode("utf-8"))
                if target_cid != orig_cid and orig_cid:
                    blob = blob.replace(orig_cid.encode("utf-8"), target_cid.encode("utf-8"))
                conv_con.execute("UPDATE trajectory_metadata_blob SET data = ? WHERE id = 'main'", (blob,))
        except Exception:
            pass

        conv_con.commit()
        conv_con.close()

        # 2. Copiar annotations si existen
        annot_file = os.path.join(temp_dir, "annotation.pbtxt")
        if os.path.exists(annot_file):
            with open(annot_file, "r", encoding="utf-8", errors="ignore") as f:
                annot_txt = f.read()
            annot_txt = re.sub(r'title:\s*"[^"]*"', f'title:"{final_title}"', annot_txt)
            dest_annot = os.path.join(ANNOTATIONS_DIR, f"{target_cid}.pbtxt")
            with open(dest_annot, "w", encoding="utf-8") as f:
                f.write(annot_txt)

        # 3. Copiar brain si existe
        brain_temp = os.path.join(temp_dir, "brain")
        dest_brain = os.path.join(BRAIN_DIR, target_cid)
        if os.path.exists(brain_temp):
            if os.path.exists(dest_brain):
                shutil.rmtree(dest_brain)
            shutil.copytree(brain_temp, dest_brain)

        # 4. Insertar fila en conversation_summaries.db
        now_str = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M:%S.%f+00:00")
        summary_dict["conversation_id"] = target_cid
        summary_dict["title"] = final_title
        summary_dict["preview"] = final_title
        summary_dict["last_modified_time"] = now_str
        summary_dict["workspace_uris"] = json.dumps(remapped_workspaces)
        summary_dict["project_id"] = target_project_id

        # Parchear raw_summary_bytes si cambiaron IDs
        if raw_summary_bytes:
            if orig_project_id and target_project_id and orig_project_id != target_project_id:
                raw_summary_bytes = raw_summary_bytes.replace(orig_project_id.encode("utf-8"), target_project_id.encode("utf-8"))
            if target_cid != orig_cid and orig_cid:
                raw_summary_bytes = raw_summary_bytes.replace(orig_cid.encode("utf-8"), target_cid.encode("utf-8"))

        con = sqlite3.connect(SUMMARIES_DB, timeout=30.0)
        col_info = con.execute("PRAGMA table_info(conversation_summaries)").fetchall()
        valid_cols = [c[1] for c in col_info]

        row_to_insert = {col: summary_dict.get(col, "") for col in valid_cols}
        row_to_insert["conversation_id"] = target_cid
        row_to_insert["raw_summary"] = raw_summary_bytes

        cols_str = ", ".join([f"`{c}`" for c in valid_cols])
        placeholders = ", ".join(["?"] * len(valid_cols))
        vals = [row_to_insert[c] for c in valid_cols]

        con.execute(f"INSERT OR REPLACE INTO conversation_summaries ({cols_str}) VALUES ({placeholders})", vals)
        con.commit()
        try:
            con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        except Exception:
            pass
        con.close()

        # Parchear agyhub_summaries_proto.pb si existe
        pb_path = os.path.join(BASE_DIR, "agyhub_summaries_proto.pb")
        if os.path.exists(pb_path) and orig_project_id and target_project_id and orig_project_id != target_project_id:
            try:
                with open(pb_path, "rb") as pbf:
                    pb_content = pbf.read()
                pb_content = pb_content.replace(orig_project_id.encode("utf-8"), target_project_id.encode("utf-8"))
                with open(pb_path, "wb") as pbf:
                    pbf.write(pb_content)
            except Exception:
                pass

        # 5. Notificar a language_server si está en ejecución
        port, csrf_token = get_live_ls_info()
        if port and csrf_token:
            load_trajectory_via_api(target_cid, port, csrf_token)
            update_annotations_via_api(target_cid, final_title, port, csrf_token)
            print(f"\n✅ ¡Listo! El chat ha sido importado y cargado en vivo.")
            print(f"   Ya aparece visible en tu barra lateral como \"{final_title}\".\n")
        else:
            print(f"\n✅ ¡Listo! El chat ha sido importado exitosamente.")
            print(f"   Aparecerá en tu barra lateral cuando abras Antigravity como \"{final_title}\".\n")

        return target_cid


def print_help():
    print("""
agychat (Antigravity Chat Manager)
==================================
Herramienta CLI para ramificar, duplicar, exportar e importar chats
de Google Antigravity entre proyectos o computadoras.

Uso:
  agychat -l | --list
  agychat -d | --duplicate "<CHAT>" [-t | --title "<TITULO>"]
  agychat -e | --export "<CHAT>" [-o | --output <ARCHIVO>]
  agychat -i | --import <ARCHIVO> [-t | --title "<TITULO>"] [-w | --workspace <RUTA>]
  agychat -h | --help

Opciones:
  -h, --help                Muestra esta ayuda.
  -l, --list                Lista todas las conversaciones registradas.
  -d, --duplicate CHAT      Duplica un chat en tiempo real por título o ID.
  -e, --export CHAT         Exporta un chat a un archivo portátil (.agychat).
  -i, --import ARCHIVO      Importa un chat desde un archivo .agychat o .tar.gz.
  -t, --title TITULO        Título personalizado al duplicar o importar.
  -o, --output ARCHIVO      Ruta del archivo de salida al exportar.
  -w, --workspace RUTA      Ruta del workspace local en la máquina de destino al importar.
      --new-id              Fuerza la creación de un nuevo UUID al importar.
      --overwrite           Sobrescribe el chat si el UUID coincide con uno existente.

Ejemplos:
  1) Listar chats disponibles:
     agychat -l
     agychat --list

  2) Duplicar con sufijo '(Copia)':
     agychat -d "Analisis Inicial"
     agychat --duplicate "Analisis Inicial"

  3) Duplicar con nuevo título:
     agychat -d "Analisis Inicial" -t "Modulo 2 - Refactor"
     agychat --duplicate "Analisis Inicial" --title "Modulo 2 - Refactor"

  4) Exportar a archivo comprimido portátil (.agychat):
     agychat -e "Analisis Inicial"
     agychat --export "Analisis Inicial" -o ~/Descargas/backup.agychat

  5) Importar en otra máquina:
     agychat -i backup.agychat
     agychat --import backup.agychat -t "Nuevo Titulo" -w ~/Proyectos/mi-repo
""")


def main():
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("-h", "--help", action="store_true")
    parser.add_argument("-l", "--list", action="store_true")
    parser.add_argument("-d", "--duplicate", nargs="?", const="", default=None)
    parser.add_argument("-e", "--export", nargs="?", const="", default=None)
    parser.add_argument("-o", "--output", default=None)
    parser.add_argument("-i", "--import", dest="import_file", default=None)
    parser.add_argument("-t", "--title", default=None)
    parser.add_argument("-w", "--workspace", default=None)
    parser.add_argument("--new-id", action="store_true")
    parser.add_argument("--overwrite", action="store_true")
    parser.add_argument("positional", nargs="*")

    args = parser.parse_args()

    if args.help or (not sys.argv[1:]):
        print_help()
        sys.exit(0)

    if args.list:
        list_conversations()
        sys.exit(0)

    # Modo Duplicar con -d / --duplicate
    if args.duplicate is not None:
        target = args.duplicate.strip()
        if not target and args.positional:
            target = args.positional.pop(0)
        if not target:
            print("❌ Error: Debes especificar el nombre o ID del chat a duplicar.")
            print("   Uso: agychat -d \"<Nombre del Chat>\" [-t \"<Nuevo Título>\"]")
            sys.exit(1)
        custom_title = args.title or (args.positional[0] if args.positional else None)
        duplicate_conversation(target, custom_title)
        sys.exit(0)

    # Modo Exportar con -e / --export
    if args.export is not None:
        target = args.export.strip()
        output_file = args.output
        if not target and args.positional:
            target = args.positional.pop(0)
        if not output_file and args.positional:
            output_file = args.positional.pop(0)
        if not target:
            print("❌ Error: Debes especificar el nombre o ID del chat a exportar.")
            print("   Uso: agychat -e \"<Nombre del Chat>\" [-o archivo.agychat]")
            sys.exit(1)
        export_conversation(target, output_file)
        sys.exit(0)

    # Modo Importar con -i / --import
    if args.import_file:
        custom_title = args.title
        if not custom_title and args.positional:
            custom_title = args.positional.pop(0)
        import_conversation(
            args.import_file,
            custom_title=custom_title,
            target_workspace=args.workspace,
            new_id=args.new_id,
            overwrite=args.overwrite,
        )
        sys.exit(0)

    # Modo Duplicar (compatibilidad posicional por defecto sin flags)
    if args.positional:
        target = args.positional[0]
        custom_title = args.title or (args.positional[1] if len(args.positional) > 1 else None)
        duplicate_conversation(target, custom_title)
        sys.exit(0)

    print_help()


if __name__ == "__main__":
    main()
