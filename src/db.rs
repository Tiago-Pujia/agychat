use crate::ls_client::{
    delete_via_api, get_live_ls_info, load_trajectory_via_api, update_annotations_via_api,
};
use crate::models::{ConversationSummary, DiskInfo};
use crate::paths::{annotations_dir, brain_dir, conversations_dir, summaries_db_path};
use crate::utils::{copy_dir_all, replace_bytes};
use chrono::Utc;
use regex::Regex;
use rusqlite::{params, Connection, OpenFlags};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

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

/// Duplica una conversación local en caliente clonando trayectoria, brain, anotaciones y SQLite.
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
    let new_title = custom_new_title
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{} (Copia)", source_title));

    let new_id = Uuid::new_v4().to_string();

    // 1. Origen y destino de la base de datos de la trayectoria
    let conv_dir = conversations_dir();
    let src_db = conv_dir.join(format!("{}.db", source_id));
    if !src_db.exists() {
        return Err(format!(
            "No se encontró el archivo de base de datos de la conversación: {}",
            src_db.display()
        ));
    }
    let dest_db = conv_dir.join(format!("{}.db", new_id));

    // Si la DB origen tiene WAL activo, hacer checkpoint para asegurar que los datos estén volcados
    if let Ok(conn_src) = Connection::open_with_flags(
        &src_db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        let _ = conn_src.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
    }

    fs::copy(&src_db, &dest_db)
        .map_err(|e| format!("Error al copiar conversation.db: {}", e))?;

    // 2. Parchear cascade_id y metadata blob en la nueva base de datos
    let conv_conn = Connection::open_with_flags(
        &dest_db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    let _ = conv_conn.execute(
        "UPDATE trajectory_meta SET cascade_id = ?1",
        params![new_id],
    );

    let blob_res: rusqlite::Result<Vec<u8>> = conv_conn.query_row(
        "SELECT data FROM trajectory_metadata_blob WHERE id = 'main'",
        [],
        |r| r.get(0),
    );

    if let Ok(mut blob) = blob_res {
        blob = replace_bytes(&blob, source_id.as_bytes(), new_id.as_bytes());
        let _ = conv_conn.execute(
            "UPDATE trajectory_metadata_blob SET data = ?1 WHERE id = 'main'",
            params![blob],
        );
    }
    let _ = conv_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    drop(conv_conn);

    // 3. Copiar archivo de anotaciones .pbtxt si existe
    let src_annot = annotations_dir().join(format!("{}.pbtxt", source_id));
    if src_annot.exists() {
        if let Ok(annot_txt) = fs::read_to_string(&src_annot) {
            if let Ok(re_title) = Regex::new(r#"title:\s*"[^"]*""#) {
                let patched_annot = re_title
                    .replace_all(&annot_txt, format!(r#"title:"{}""#, new_title))
                    .to_string();
                let dest_annot = annotations_dir().join(format!("{}.pbtxt", new_id));
                let _ = fs::write(dest_annot, patched_annot);
            }
        }
    }

    // 4. Copiar carpeta de artefactos brain/<source_id> si existe
    let src_brain = brain_dir().join(&source_id);
    let dest_brain = brain_dir().join(&new_id);
    if src_brain.exists() {
        if dest_brain.exists() {
            let _ = fs::remove_dir_all(&dest_brain);
        }
        let _ = copy_dir_all(&src_brain, &dest_brain);
    }

    // 5. Clonar e insertar fila en conversation_summaries.db
    let sum_conn = Connection::open_with_flags(
        summaries_db_path(),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    sum_conn
        .busy_timeout(Duration::from_secs(30))
        .map_err(|e| e.to_string())?;

    // Obtener nombres de columnas de conversation_summaries
    let mut col_stmt = sum_conn
        .prepare("PRAGMA table_info(conversation_summaries)")
        .map_err(|e| e.to_string())?;

    let valid_cols: Vec<String> = col_stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();
    drop(col_stmt);

    let select_sql = format!(
        "SELECT {} FROM conversation_summaries WHERE conversation_id = ?1",
        valid_cols
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut select_stmt = sum_conn.prepare(&select_sql).map_err(|e| e.to_string())?;
    let now_str = Utc::now().format("%Y-%m-%d %H:%M:%S.%6f+00:00").to_string();

    let mut new_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    let _ = select_stmt.query_row(params![source_id], |row| {
        for (idx, col) in valid_cols.iter().enumerate() {
            match col.as_str() {
                "conversation_id" => {
                    new_values.push(Box::new(new_id.clone()));
                }
                "title" | "preview" => {
                    new_values.push(Box::new(new_title.clone()));
                }
                "last_modified_time" => {
                    new_values.push(Box::new(now_str.clone()));
                }
                "status" => {
                    new_values.push(Box::new("CASCADE_RUN_STATUS_IDLE".to_string()));
                }
                "parent_conversation_id" => {
                    new_values.push(Box::new(String::new()));
                }
                "nesting_depth" => {
                    new_values.push(Box::new(0i64));
                }
                "raw_summary" => {
                    let mut blob: Option<Vec<u8>> = row.get(idx).unwrap_or(None);
                    if let Some(ref mut b) = blob {
                        *b = replace_bytes(b, source_id.as_bytes(), new_id.as_bytes());
                    }
                    new_values.push(Box::new(blob));
                }
                _ => {
                    let val: rusqlite::types::Value = row.get(idx)?;
                    match val {
                        rusqlite::types::Value::Null => new_values.push(Box::new(None::<String>)),
                        rusqlite::types::Value::Integer(i) => new_values.push(Box::new(i)),
                        rusqlite::types::Value::Real(f) => new_values.push(Box::new(f)),
                        rusqlite::types::Value::Text(s) => new_values.push(Box::new(s)),
                        rusqlite::types::Value::Blob(b) => new_values.push(Box::new(b)),
                    }
                }
            }
        }
        Ok(())
    });
    drop(select_stmt);

    if !new_values.is_empty() {
        let cols_str = valid_cols
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = valid_cols
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let insert_sql = format!(
            "INSERT OR REPLACE INTO conversation_summaries ({}) VALUES ({})",
            cols_str, placeholders
        );

        let mut insert_stmt = sum_conn.prepare(&insert_sql).map_err(|e| e.to_string())?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            new_values.iter().map(|b| b.as_ref()).collect();
        insert_stmt
            .execute(rusqlite_params.as_slice())
            .map_err(|e| format!("Error insertando conversación duplicada: {}", e))?;
        drop(insert_stmt);
    }

    let _ = sum_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    drop(sum_conn);

    // 6. Notificar a language_server si está en ejecución para cargar en vivo
    if let Some(live) = get_live_ls_info() {
        load_trajectory_via_api(&new_id, live.port, &live.csrf_token);
        update_annotations_via_api(&new_id, &new_title, live.port, &live.csrf_token);
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
