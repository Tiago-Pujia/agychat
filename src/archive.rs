use crate::db::find_conversation;
use crate::ls_client::{get_live_ls_info, load_trajectory_via_api, update_annotations_via_api};
use crate::models::ArchiveManifest;
use crate::paths::{
    agyhub_summaries_pb_path, annotations_dir, base_dir, brain_dir, conversations_dir,
    home_dir, summaries_db_path,
};
use crate::utils::{get_known_projects, replace_bytes};
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::Value;
use std::fs::{self, File};

use std::path::{Path, PathBuf};
use std::time::Duration;
use tar::{Archive, Builder, Header};
use uuid::Uuid;

/// Empaqueta una conversación completa en un archivo comprimido portable .agychat (tar.gz).
pub fn export_conversation(
    source_id_or_title: &str,
    output_path: Option<&str>,
) -> Result<PathBuf, String> {
    let row = find_conversation(source_id_or_title)?
        .ok_or_else(|| format!("No se encontró ningún chat que coincida con '{}'", source_id_or_title))?;

    let cid = row.conversation_id;
    let title = if row.title.is_empty() {
        "Chat_Sin_Titulo".to_string()
    } else {
        row.title
    };
    let step_count = row.step_count;

    let db_path = conversations_dir().join(format!("{}.db", cid));
    if !db_path.exists() {
        return Err(format!(
            "No se encontró el archivo de base de datos de la conversación: {}",
            db_path.display()
        ));
    }

    // Ruta destino del archivo .agychat
    let out_file: PathBuf = match output_path {
        Some(p) => PathBuf::from(p),
        None => {
            let re_clean = Regex::new(r"[^\w\s-]").map_err(|e| e.to_string())?;
            let re_spaces = Regex::new(r"[-\s]+").map_err(|e| e.to_string())?;
            let mut clean_title = re_clean.replace_all(&title, "").to_string();
            clean_title = re_spaces.replace_all(&clean_title, "_").trim().to_string();
            if clean_title.is_empty() {
                clean_title = "antigravity_chat".to_string();
            }
            PathBuf::from(format!("{}.agychat", clean_title))
        }
    };

    let abs_out = if out_file.is_absolute() {
        out_file
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(out_file)
    };

    if let Some(parent) = abs_out.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Obtener campos de conversation_summaries y raw_summary blob
    let conn = Connection::open_with_flags(
        summaries_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT * FROM conversation_summaries WHERE conversation_id = ?1")
        .map_err(|e| e.to_string())?;

    let column_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut summary_map = serde_json::Map::new();
    let mut raw_summary_bytes: Option<Vec<u8>> = None;

    let _ = stmt.query_row(params![cid], |r| {
        for (idx, col) in column_names.iter().enumerate() {
            if col == "raw_summary" {
                raw_summary_bytes = r.get::<_, Option<Vec<u8>>>(idx).unwrap_or(None);
            } else {
                let val_res = r.get::<_, rusqlite::types::Value>(idx);
                if let Ok(val) = val_res {
                    let json_val = match val {
                        rusqlite::types::Value::Null => Value::Null,
                        rusqlite::types::Value::Integer(i) => Value::from(i),
                        rusqlite::types::Value::Real(f) => Value::from(f),
                        rusqlite::types::Value::Text(s) => Value::from(s),
                        rusqlite::types::Value::Blob(b) => Value::from(format!("{:?}", b)),
                    };
                    summary_map.insert(col.clone(), json_val);
                }
            }
        }
        Ok(())
    });

    let orig_workspaces: Vec<String> = if !row.workspace_uris.is_empty() {
        if row.workspace_uris.starts_with('[') {
            serde_json::from_str(&row.workspace_uris).unwrap_or_else(|_| vec![row.workspace_uris.clone()])
        } else {
            vec![row.workspace_uris.clone()]
        }
    } else {
        vec![]
    };

    let manifest = ArchiveManifest {
        format: "antigravity-chat-archive".to_string(),
        version: 1,
        conversation_id: cid.clone(),
        title: title.clone(),
        step_count,
        workspace_uris: orig_workspaces,
        project_id: row.project_id.clone(),
        source_home: home_dir().to_string_lossy().to_string(),
        source_user: std::env::var("USER").unwrap_or_default(),
        last_modified_time: row.last_modified_time.clone(),
        exported_at: Utc::now().to_rfc3339(),
    };

    // Crear el archivo tar.gz
    let file = File::create(&abs_out).map_err(|e| e.to_string())?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    // 1. manifest.json
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let mut h_manifest = Header::new_gnu();
    h_manifest.set_size(manifest_bytes.len() as u64);
    h_manifest.set_mode(0o644);
    h_manifest.set_cksum();
    tar.append_data(&mut h_manifest, "manifest.json", &manifest_bytes[..])
        .map_err(|e| e.to_string())?;

    // 2. summary.json
    let summary_bytes = serde_json::to_vec_pretty(&summary_map).map_err(|e| e.to_string())?;
    let mut h_summary = Header::new_gnu();
    h_summary.set_size(summary_bytes.len() as u64);
    h_summary.set_mode(0o644);
    h_summary.set_cksum();
    tar.append_data(&mut h_summary, "summary.json", &summary_bytes[..])
        .map_err(|e| e.to_string())?;

    // 3. raw_summary.bin (si existe)
    if let Some(raw_bytes) = raw_summary_bytes {
        let mut h_raw = Header::new_gnu();
        h_raw.set_size(raw_bytes.len() as u64);
        h_raw.set_mode(0o644);
        h_raw.set_cksum();
        tar.append_data(&mut h_raw, "raw_summary.bin", &raw_bytes[..])
            .map_err(|e| e.to_string())?;
    }

    // 4. conversation.db
    let mut db_file = File::open(&db_path).map_err(|e| e.to_string())?;
    tar.append_file("conversation.db", &mut db_file)
        .map_err(|e| e.to_string())?;

    // 5. annotation.pbtxt (si existe)
    let pbtxt_path = annotations_dir().join(format!("{}.pbtxt", cid));
    if pbtxt_path.exists() {
        let mut pbtxt_file = File::open(&pbtxt_path).map_err(|e| e.to_string())?;
        tar.append_file("annotation.pbtxt", &mut pbtxt_file)
            .map_err(|e| e.to_string())?;
    }

    // 6. brain/ (si existe)
    let brain_path = brain_dir().join(&cid);
    if brain_path.exists() {
        tar.append_dir_all("brain", &brain_path)
            .map_err(|e| e.to_string())?;
    }

    tar.finish().map_err(|e| e.to_string())?;

    Ok(abs_out)
}

/// Importa una conversación desde un archivo .agychat adaptando rutas y cargando en caliente.
pub fn import_conversation(
    archive_path: &str,
    custom_title: Option<&str>,
    target_workspace: Option<&str>,
    new_id: bool,
    overwrite: bool,
) -> Result<String, String> {
    let arc_path = Path::new(archive_path);
    if !arc_path.exists() {
        return Err(format!("El archivo '{}' no existe", archive_path));
    }

    let b_dir = base_dir();
    if !b_dir.exists() {
        let _ = fs::create_dir_all(&b_dir);
    }
    let _ = fs::create_dir_all(conversations_dir());
    let _ = fs::create_dir_all(annotations_dir());
    let _ = fs::create_dir_all(brain_dir());

    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let temp_path = temp_dir.path();

    // Descomprimir tar.gz
    let tar_file = File::open(arc_path).map_err(|e| e.to_string())?;
    let tar_gz = GzDecoder::new(tar_file);
    let mut archive = Archive::new(tar_gz);
    archive.unpack(temp_path).map_err(|e| format!("Error al descomprimir archivo .agychat: {}", e))?;

    let manifest_file = temp_path.join("manifest.json");
    let summary_file = temp_path.join("summary.json");
    let db_file = temp_path.join("conversation.db");

    if !manifest_file.exists() || !db_file.exists() {
        return Err("El archivo no tiene la estructura válida de un chat de Antigravity (falta manifest.json o conversation.db)".to_string());
    }

    let manifest_content = fs::read_to_string(&manifest_file).map_err(|e| e.to_string())?;
    let manifest: ArchiveManifest = serde_json::from_str(&manifest_content).map_err(|e| e.to_string())?;

    let mut summary_map: serde_json::Map<String, Value> = if summary_file.exists() {
        let s_content = fs::read_to_string(&summary_file).map_err(|e| e.to_string())?;
        serde_json::from_str(&s_content).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    let mut raw_summary_bytes: Option<Vec<u8>> = {
        let raw_file = temp_path.join("raw_summary.bin");
        if raw_file.exists() {
            fs::read(&raw_file).ok()
        } else {
            None
        }
    };

    let orig_cid = manifest.conversation_id.clone();
    let orig_title = manifest.title.clone();
    let final_title = custom_title.unwrap_or(&orig_title).to_string();

    // Comprobar si ya existe el UUID en conversation_summaries.db
    let mut existing = false;
    if let Ok(conn) = Connection::open_with_flags(
        summaries_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        let found: rusqlite::Result<String> = conn.query_row(
            "SELECT conversation_id FROM conversation_summaries WHERE conversation_id = ?1",
            params![orig_cid],
            |r| r.get(0),
        );
        existing = found.is_ok();
    }

    let target_cid = if new_id {
        Uuid::new_v4().to_string()
    } else if existing {
        if overwrite {
            orig_cid.clone()
        } else {
            Uuid::new_v4().to_string()
        }
    } else {
        orig_cid.clone()
    };

    let known_projects = get_known_projects();

    // 1. Resolver Workspace y Project ID en la máquina destino
    let mut remapped_workspaces: Vec<String> = Vec::new();
    let mut target_project_id = String::new();

    if let Some(tw) = target_workspace.filter(|s| !s.trim().is_empty()) {
        let tw_trimmed = tw.trim();
        // Verificar si coincide con el nombre o ID de un proyecto conocido (ej. "LibreMIEL")
        if let Some(p) = known_projects.iter().find(|p| {
            p.name.eq_ignore_ascii_case(tw_trimmed)
                || p.id.eq_ignore_ascii_case(tw_trimmed)
        }) {
            target_project_id = p.id.clone();
            remapped_workspaces.push(p.folder_uri.clone());
        } else {
            let p = Path::new(tw_trimmed);
            let abs = if p.is_absolute() {
                p.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(p)
            };
            let uri = format!("file://{}", abs.to_string_lossy());
            remapped_workspaces.push(uri.clone());

            let norm_uri = uri.trim_end_matches('/').to_lowercase();
            if let Some(kp) = known_projects
                .iter()
                .find(|kp| kp.folder_uri.trim_end_matches('/').to_lowercase() == norm_uri)
            {
                target_project_id = kp.id.clone();
            }
        }
    } else {
        // Si no se especificó target_workspace, verificar si manifest.project_id coincide con un proyecto local
        if !manifest.project_id.is_empty() {
            if let Some(p) = known_projects.iter().find(|p| p.id == manifest.project_id) {
                target_project_id = p.id.clone();
                if !p.folder_uri.is_empty() {
                    remapped_workspaces.push(p.folder_uri.clone());
                }
            }
        }

        // Si todavía no se resolvió workspace, verificar si la carpeta del workspace coincide con un proyecto conocido
        if remapped_workspaces.is_empty() {
            for ws in &manifest.workspace_uris {
                let ws_name = ws.trim_end_matches('/').split('/').last().unwrap_or_default();
                if !ws_name.is_empty() {
                    if let Some(p) = known_projects.iter().find(|p| p.name.eq_ignore_ascii_case(ws_name)) {
                        target_project_id = p.id.clone();
                        remapped_workspaces.push(p.folder_uri.clone());
                        break;
                    }
                }
            }
        }

        // Si sigue sin resolverse, aplicar remapeo de $HOME
        if remapped_workspaces.is_empty() {
            let source_home = &manifest.source_home;
            let target_home = home_dir().to_string_lossy().to_string();

            for ws in &manifest.workspace_uris {
                if !source_home.is_empty() && source_home != &target_home {
                    let remapped = ws.replace(
                        &format!("file://{}", source_home),
                        &format!("file://{}", target_home),
                    );
                    remapped_workspaces.push(remapped);
                } else {
                    remapped_workspaces.push(ws.clone());
                }
            }

            // Si la ruta no existe en este equipo, verificar si el CWD actual tiene .git
            if !remapped_workspaces.is_empty() {
                let check_path = remapped_workspaces[0].replace("file://", "");
                if !Path::new(&check_path).exists() {
                    if let Ok(cwd) = std::env::current_dir() {
                        if cwd.join(".git").exists() {
                            remapped_workspaces = vec![format!("file://{}", cwd.to_string_lossy())];
                        }
                    }
                }
            }
        }
    }

    // 2. Si target_project_id aún no se resolvió, buscar por URI de workspace o en chats existentes
    if target_project_id.is_empty() && !remapped_workspaces.is_empty() {
        let norm_uri = remapped_workspaces[0].trim_end_matches('/').to_lowercase();
        if let Some(kp) = known_projects
            .iter()
            .find(|kp| kp.folder_uri.trim_end_matches('/').to_lowercase() == norm_uri)
        {
            target_project_id = kp.id.clone();
        } else if let Ok(conn) = Connection::open_with_flags(
            summaries_db_path(),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let pattern = format!("%{}%", remapped_workspaces[0]);
            let found: rusqlite::Result<String> = conn.query_row(
                "SELECT project_id FROM conversation_summaries WHERE workspace_uris LIKE ?1 AND project_id != '' LIMIT 1",
                params![pattern],
                |r| r.get(0),
            );
            if let Ok(p_id) = found {
                target_project_id = p_id;
            }
        }
    }

    if target_project_id.is_empty() {
        target_project_id = if !manifest.project_id.is_empty() {
            manifest.project_id.clone()
        } else {
            Uuid::new_v4().to_string()
        };
    }

    let orig_project_id = manifest.project_id.clone();

    // 3. Copiar base de datos de la conversación y actualizar cascade_id y metadata blob
    let dest_db = conversations_dir().join(format!("{}.db", target_cid));
    fs::copy(&db_file, &dest_db).map_err(|e| format!("Error copiando conversation.db: {}", e))?;

    let conv_conn = Connection::open_with_flags(
        &dest_db,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    let _ = conv_conn.execute(
        "UPDATE trajectory_meta SET cascade_id = ?1",
        params![target_cid],
    );

    // Parchear project_id y cascade_id en trajectory_metadata_blob si cambiaron
    let blob_res: rusqlite::Result<Vec<u8>> = conv_conn.query_row(
        "SELECT data FROM trajectory_metadata_blob WHERE id = 'main'",
        [],
        |r| r.get(0),
    );

    if let Ok(mut blob) = blob_res {
        if !orig_project_id.is_empty()
            && !target_project_id.is_empty()
            && orig_project_id != target_project_id
        {
            blob = replace_bytes(&blob, orig_project_id.as_bytes(), target_project_id.as_bytes());
        }
        if !orig_cid.is_empty() && orig_cid != target_cid {
            blob = replace_bytes(&blob, orig_cid.as_bytes(), target_cid.as_bytes());
        }
        let _ = conv_conn.execute(
            "UPDATE trajectory_metadata_blob SET data = ?1 WHERE id = 'main'",
            params![blob],
        );
    }
    drop(conv_conn);

    // 4. Copiar annotations si existen
    let annot_file = temp_path.join("annotation.pbtxt");
    if annot_file.exists() {
        if let Ok(mut annot_txt) = fs::read_to_string(&annot_file) {
            let re_title = Regex::new(r#"title:\s*"[^"]*""#).map_err(|e| e.to_string())?;
            annot_txt = re_title
                .replace_all(&annot_txt, format!(r#"title:"{}""#, final_title))
                .to_string();
            let dest_annot = annotations_dir().join(format!("{}.pbtxt", target_cid));
            let _ = fs::write(dest_annot, annot_txt);
        }
    }

    // 5. Copiar carpeta brain si existe
    let brain_temp = temp_path.join("brain");
    let dest_brain = brain_dir().join(&target_cid);
    if brain_temp.exists() {
        if dest_brain.exists() {
            let _ = fs::remove_dir_all(&dest_brain);
        }
        copy_dir_all(&brain_temp, &dest_brain).map_err(|e| format!("Error copiando brain: {}", e))?;
    }

    // 6. Insertar fila en conversation_summaries.db
    let now_str = Utc::now().format("%Y-%m-%d %H:%M:%S.%6f+00:00").to_string();
    summary_map.insert("conversation_id".to_string(), Value::String(target_cid.clone()));
    summary_map.insert("title".to_string(), Value::String(final_title.clone()));
    summary_map.insert("preview".to_string(), Value::String(final_title.clone()));
    summary_map.insert("last_modified_time".to_string(), Value::String(now_str));
    summary_map.insert(
        "workspace_uris".to_string(),
        Value::String(serde_json::to_string(&remapped_workspaces).unwrap_or_default()),
    );
    summary_map.insert("project_id".to_string(), Value::String(target_project_id.clone()));

    // Parchear raw_summary_bytes si cambiaron IDs
    if let Some(ref mut raw_b) = raw_summary_bytes {
        if !orig_project_id.is_empty()
            && !target_project_id.is_empty()
            && orig_project_id != target_project_id
        {
            *raw_b = replace_bytes(raw_b, orig_project_id.as_bytes(), target_project_id.as_bytes());
        }
        if !orig_cid.is_empty() && orig_cid != target_cid {
            *raw_b = replace_bytes(raw_b, orig_cid.as_bytes(), target_cid.as_bytes());
        }
    }

    // Abrir conexión SQLite para inserción dinámica
    let sum_conn = Connection::open_with_flags(
        summaries_db_path(),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    sum_conn.busy_timeout(Duration::from_secs(30)).map_err(|e| e.to_string())?;

    let mut col_stmt = sum_conn
        .prepare("PRAGMA table_info(conversation_summaries)")
        .map_err(|e| e.to_string())?;

    let valid_cols: Vec<String> = col_stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();

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

    let sql = format!(
        "INSERT OR REPLACE INTO conversation_summaries ({}) VALUES ({})",
        cols_str, placeholders
    );

    let mut insert_stmt = sum_conn.prepare(&sql).map_err(|e| e.to_string())?;

    let mut sql_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    for col in &valid_cols {
        if col == "raw_summary" {
            sql_params.push(Box::new(raw_summary_bytes.clone()));
        } else {
            let v = summary_map.get(col);
            match v {
                Some(Value::String(s)) => sql_params.push(Box::new(s.clone())),
                Some(Value::Number(n)) => {
                    if let Some(i) = n.as_i64() {
                        sql_params.push(Box::new(i));
                    } else if let Some(f) = n.as_f64() {
                        sql_params.push(Box::new(f));
                    } else {
                        sql_params.push(Box::new(None::<String>));
                    }
                }
                Some(Value::Bool(b)) => sql_params.push(Box::new(*b)),
                _ => sql_params.push(Box::new(None::<String>)),
            }
        }
    }

    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|b| b.as_ref()).collect();
    insert_stmt
        .execute(rusqlite_params.as_slice())
        .map_err(|e| format!("Error insertando en conversation_summaries: {}", e))?;

    drop(insert_stmt);
    drop(col_stmt);
    let _ = sum_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    // 7. Parchear agyhub_summaries_proto.pb si existe y cambió project_id
    let pb_path = agyhub_summaries_pb_path();
    if pb_path.exists()
        && !orig_project_id.is_empty()
        && !target_project_id.is_empty()
        && orig_project_id != target_project_id
    {
        if let Ok(pb_content) = fs::read(&pb_path) {
            let patched = replace_bytes(&pb_content, orig_project_id.as_bytes(), target_project_id.as_bytes());
            let _ = fs::write(&pb_path, patched);
        }
    }

    // 8. Notificar a language_server si está en ejecución
    if let Some(live) = get_live_ls_info() {
        load_trajectory_via_api(&target_cid, live.port, &live.csrf_token);
        update_annotations_via_api(&target_cid, &final_title, live.port, &live.csrf_token);
    }

    Ok(target_cid)
}

/// Helper para copiar un directorio de forma recursiva.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
