use crate::ls_client::{delete_via_api, fork_via_api, get_live_ls_info};
use crate::models::{ConversationSummary, DiskInfo};
use crate::paths::{annotations_dir, brain_dir, conversations_dir, summaries_db_path};
use rusqlite::{params, Connection, OpenFlags};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

/// Abre una conexión a conversation_summaries.db configurando busy_timeout.
pub fn get_connection() -> rusqlite::Result<Connection> {
    let path = summaries_db_path();
    if !path.exists() {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some("No se encontró la base de datos de Antigravity".to_string()),
        ));
    }

    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_secs(30))?;
    Ok(conn)
}

fn val_to_string(val: rusqlite::types::Value) -> String {
    match val {
        rusqlite::types::Value::Text(s) => s,
        rusqlite::types::Value::Integer(i) => i.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        _ => String::new(),
    }
}

fn val_to_i64(val: rusqlite::types::Value) -> i64 {
    match val {
        rusqlite::types::Value::Integer(i) => i,
        rusqlite::types::Value::Real(f) => f as i64,
        rusqlite::types::Value::Text(s) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

/// Recupera todas las conversaciones registradas ordenadas por última modificación descendente.
pub fn get_all_conversations() -> Result<Vec<ConversationSummary>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT conversation_id, title, step_count, last_modified_time, workspace_uris,
                    project_id, status, last_user_input_time, last_user_input_step_index
             FROM conversation_summaries
             ORDER BY last_modified_time DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ConversationSummary {
                conversation_id: val_to_string(row.get(0)?),
                title: val_to_string(row.get(1)?),
                step_count: val_to_i64(row.get(2)?),
                last_modified_time: val_to_string(row.get(3)?),
                workspace_uris: val_to_string(row.get(4)?),
                project_id: val_to_string(row.get(5)?),
                status: val_to_string(row.get(6)?),
                last_user_input_time: val_to_string(row.get(7)?),
                last_user_input_step_index: val_to_string(row.get(8)?),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            list.push(item);
        }
    }
    Ok(list)
}

/// Busca una conversación por UUID exacto o por coincidencia parcial en el título.
pub fn find_conversation(source_id_or_title: &str) -> Result<Option<ConversationSummary>, String> {
    let conn = get_connection().map_err(|e| e.to_string())?;

    let like_pattern = format!("%{}%", source_id_or_title);
    let mut stmt = conn
        .prepare(
            "SELECT conversation_id, title, step_count, last_modified_time, workspace_uris,
                    project_id, status, last_user_input_time, last_user_input_step_index
             FROM conversation_summaries
             WHERE conversation_id = ?1 OR title LIKE ?2
             ORDER BY last_modified_time DESC LIMIT 1",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query(params![source_id_or_title, like_pattern])
        .map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(ConversationSummary {
            conversation_id: val_to_string(row.get(0).map_err(|e| e.to_string())?),
            title: val_to_string(row.get(1).map_err(|e| e.to_string())?),
            step_count: val_to_i64(row.get(2).map_err(|e| e.to_string())?),
            last_modified_time: val_to_string(row.get(3).map_err(|e| e.to_string())?),
            workspace_uris: val_to_string(row.get(4).map_err(|e| e.to_string())?),
            project_id: val_to_string(row.get(5).map_err(|e| e.to_string())?),
            status: val_to_string(row.get(6).map_err(|e| e.to_string())?),
            last_user_input_time: val_to_string(row.get(7).map_err(|e| e.to_string())?),
            last_user_input_step_index: val_to_string(row.get(8).map_err(|e| e.to_string())?),
        }))
    } else {
        Ok(None)
    }
}

/// Calcula el espacio real en disco ocupado por una conversación (DBs, pbtxt y carpeta brain).
pub fn get_conversation_disk_info(cid: &str) -> DiskInfo {
    let mut info = DiskInfo::default();

    // 1. Archivos SQLite de la conversación
    let conv_dir = conversations_dir();
    for ext in &["", "-wal", "-shm"] {
        let path = if ext.is_empty() {
            conv_dir.join(format!("{}.db", cid))
        } else {
            conv_dir.join(format!("{}.db{}", cid, ext))
        };
        if path.exists() {
            if let Ok(meta) = fs::metadata(&path) {
                let sz = meta.len();
                info.db_bytes += sz;
                info.total_bytes += sz;
            }
        }
    }

    // 2. Archivo de anotaciones pbtxt
    let annot_f = annotations_dir().join(format!("{}.pbtxt", cid));
    if annot_f.exists() {
        if let Ok(meta) = fs::metadata(&annot_f) {
            let sz = meta.len();
            info.annot_bytes += sz;
            info.total_bytes += sz;
        }
    }

    // 3. Carpeta de artefactos brain/<cid>
    let brain_d = brain_dir().join(cid);
    if brain_d.exists() {
        visit_dirs(&brain_d, &mut |entry| {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let sz = meta.len();
                    info.brain_bytes += sz;
                    info.brain_files += 1;
                    info.total_bytes += sz;
                }
            }
        });
    }

    info
}

/// Helper recursivo para calcular tamaño de carpetas.
fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&fs::DirEntry)) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb);
            } else {
                cb(&entry);
            }
        }
    }
}

/// Calcula el almacenamiento total ocupado por todas las conversaciones en ~/.gemini/antigravity.
pub fn get_total_disk_usage() -> (u64, usize) {
    let mut total_bytes = 0u64;
    let mut total_files = 0usize;

    for dir in &[conversations_dir(), annotations_dir(), brain_dir()] {
        if dir.exists() {
            visit_dirs(dir, &mut |entry| {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total_bytes += meta.len();
                        total_files += 1;
                    }
                }
            });
        }
    }

    if let Ok(meta) = fs::metadata(summaries_db_path()) {
        total_bytes += meta.len();
        total_files += 1;
    }

    (total_bytes, total_files)
}

/// Duplica una conversación local en caliente a través de la API de Antigravity.
pub fn duplicate_conversation(
    source_id_or_title: &str,
    custom_new_title: Option<&str>,
) -> Result<String, String> {
    let row = find_conversation(source_id_or_title)?
        .ok_or_else(|| format!("No se encontró ningún chat que coincida con '{}'", source_id_or_title))?;

    let source_id = row.conversation_id;
    let source_title = if row.title.is_empty() {
        "(Sin título)".to_string()
    } else {
        row.title
    };
    let step_count = row.step_count;
    let new_title = custom_new_title
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{} (Copia)", source_title));

    let live_info = get_live_ls_info()
        .ok_or_else(|| "Antigravity no parece estar en ejecución. Abre la aplicación antes de duplicar.".to_string())?;

    let new_id = fork_via_api(
        &source_id,
        &new_title,
        live_info.port,
        &live_info.csrf_token,
        step_count,
    )?;

    // Polling breve (hasta 40 intentos de 50ms = 2s) para confirmar la persistencia en conversation_summaries.db
    for _ in 0..40 {
        if let Ok(conn) = get_connection() {
            let found: rusqlite::Result<i32> = conn.query_row(
                "SELECT 1 FROM conversation_summaries WHERE conversation_id = ?1",
                params![new_id],
                |r| r.get(0),
            );
            if found.is_ok() {
                break;
            }
        }
        sleep(Duration::from_millis(50));
    }

    Ok(new_id)
}

/// Elimina permanentemente una conversación tanto en memoria como de SQLite y disco.
pub fn delete_conversation(source_id_or_title: &str, force: bool) -> Result<bool, String> {
    let row = find_conversation(source_id_or_title)?
        .ok_or_else(|| format!("No se encontró ningún chat que coincida con '{}'", source_id_or_title))?;

    let cid = row.conversation_id;
    let title = if row.title.is_empty() {
        "(Sin título)".to_string()
    } else {
        row.title
    };

    if !force {
        print!("⚠️  ¿Estás seguro de que deseas eliminar permanentemente \"{}\"? [s/N]: ", title);
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return Ok(false);
        }
        let trimmed = input.trim().to_lowercase();
        if !["s", "si", "sí", "y", "yes"].contains(&trimmed.as_str()) {
            println!("Operación cancelada.");
            return Ok(false);
        }
    }

    // 1. Notificar a language_server para descargar de memoria
    if let Some(live) = get_live_ls_info() {
        delete_via_api(&cid, live.port, &live.csrf_token);
    }

    // 2. Eliminar fila de conversation_summaries.db
    let conn = get_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM conversation_summaries WHERE conversation_id = ?1",
        params![cid],
    )
    .map_err(|e| e.to_string())?;

    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    // 3. Eliminar archivos de disco
    let conv_dir = conversations_dir();
    for ext in &["", "-wal", "-shm"] {
        let f = if ext.is_empty() {
            conv_dir.join(format!("{}.db", cid))
        } else {
            conv_dir.join(format!("{}.db{}", cid, ext))
        };
        let _ = fs::remove_file(f);
    }

    let annot_f = annotations_dir().join(format!("{}.pbtxt", cid));
    let _ = fs::remove_file(annot_f);

    let brain_d = brain_dir().join(&cid);
    if brain_d.exists() {
        let _ = fs::remove_dir_all(brain_d);
    }

    Ok(true)
}
