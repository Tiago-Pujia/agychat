use chrono::{DateTime, Utc};
use std::path::Path;

/// Convierte número de bytes a formato legible (B, KB, MB, GB).
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b < KB {
        format!("{} B", bytes)
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

/// Convierte una cadena de timestamp a formato amigable relativo (en español).
pub fn format_relative_time(dt_str: &str) -> String {
    if dt_str.is_empty() {
        return "Desconocido".to_string();
    }

    // Limpieza preliminar: cortar en '+' o '.' y normalizar 'T'
    let clean = dt_str
        .split('+')
        .next()
        .unwrap_or(dt_str)
        .split('.')
        .next()
        .unwrap_or(dt_str)
        .replace('T', " ");

    let dt = chrono::NaiveDateTime::parse_from_str(clean.trim(), "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(clean.trim(), "%Y-%m-%d %H:%M"));

    match dt {
        Ok(naive) => {
            let parsed_utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
            let now = Utc::now();
            let duration = now.signed_duration_since(parsed_utc);

            let days = duration.num_days();
            let seconds = duration.num_seconds();

            if days == 0 {
                if seconds < 60 {
                    "Hace unos instantes".to_string()
                } else if seconds < 3600 {
                    format!("Hace {} min", seconds / 60)
                } else {
                    format!("Hoy {}", naive.format("%H:%M"))
                }
            } else if days == 1 {
                format!("Ayer {}", naive.format("%H:%M"))
            } else if days < 7 {
                format!("Hace {} días", days)
            } else {
                naive.format("%d/%m/%Y").to_string()
            }
        }
        Err(_) => {
            if dt_str.len() > 16 {
                dt_str[..16].replace('T', " ")
            } else {
                dt_str.to_string()
            }
        }
    }
}

/// Reemplazo seguro sobre slices binarios (`&[u8]`) sin decodificación UTF-8.
/// Indispensable para no corromper blobs Protobuf en SQLite.
pub fn replace_bytes(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() || needle == replacement {
        return haystack.to_vec();
    }

    let mut result = Vec::with_capacity(haystack.len());
    let mut i = 0;
    while i <= haystack.len().saturating_sub(needle.len()) {
        if &haystack[i..i + needle.len()] == needle {
            result.extend_from_slice(replacement);
            i += needle.len();
        } else {
            result.push(haystack[i]);
            i += 1;
        }
    }
    result.extend_from_slice(&haystack[i..]);
    result
}

/// Extrae nombre amigable del proyecto y ruta local a partir de la URI de workspace.
pub fn extract_workspace_info(ws_raw: &str) -> (String, String) {
    if ws_raw.is_empty() {
        return ("(Sin workspace)".to_string(), String::new());
    }

    let uri = if ws_raw.starts_with('[') {
        if let Ok(uris) = serde_json::from_str::<Vec<String>>(ws_raw) {
            uris.into_iter().next().unwrap_or_default()
        } else {
            ws_raw.to_string()
        }
    } else {
        ws_raw.to_string()
    };

    if uri.is_empty() {
        return ("(Sin workspace)".to_string(), String::new());
    }

    let unquoted = urlencoding::decode(&uri)
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| uri.clone());
    let local_path = unquoted.replace("file://", "");
    let display_name = Path::new(&local_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&local_path)
        .to_string();

    (display_name, local_path)
}

/// Lee todos los proyectos registrados en ~/.gemini/config/projects/*.json
pub fn get_known_projects() -> Vec<crate::models::ProjectInfo> {
    let mut projects = Vec::new();
    let p_dir = crate::paths::projects_dir();
    if !p_dir.exists() {
        return projects;
    }

    if let Ok(entries) = std::fs::read_dir(p_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                        let id = data.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

                        let mut folder_uri = String::new();
                        if let Some(res_arr) = data
                            .get("projectResources")
                            .and_then(|pr| pr.get("resources"))
                            .and_then(|r| r.as_array())
                        {
                            for r in res_arr {
                                let f_uri = r
                                    .get("folderUri")
                                    .or_else(|| r.get("gitFolder").and_then(|gf| gf.get("folderUri")))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or_default();
                                if !f_uri.is_empty() {
                                    folder_uri = f_uri.to_string();
                                    break;
                                }
                            }
                        }

                        if !id.is_empty() {
                            projects.push(crate::models::ProjectInfo {
                                id,
                                name,
                                folder_uri,
                            });
                        }
                    }
                }
            }
        }
    }

    projects
}

/// Resuelve el nombre amigable de proyecto y la ruta local usando project_id y workspace_uris.
pub fn resolve_project_and_workspace(
    project_id: &str,
    ws_raw: &str,
    known_projects: &[crate::models::ProjectInfo],
) -> (String, String) {
    let (ws_name, local_path) = extract_workspace_info(ws_raw);

    // 1. Coincidencia por project_id
    if !project_id.is_empty() {
        if let Some(p) = known_projects.iter().find(|p| p.id == project_id) {
            let path = if !local_path.is_empty() {
                local_path
            } else {
                p.folder_uri.replace("file://", "")
            };
            return (p.name.clone(), path);
        }
    }

    // 2. Coincidencia por folder_uri
    if !ws_raw.is_empty() {
        let norm_uri = ws_raw.trim_matches('[').trim_matches(']').trim_matches('"').trim_end_matches('/').to_lowercase();
        if let Some(p) = known_projects.iter().find(|p| p.folder_uri.trim_end_matches('/').to_lowercase() == norm_uri) {
            return (p.name.clone(), local_path);
        }
    }

    (ws_name, local_path)
}

/// Copia un directorio recursivamente.
pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024 * 5), "5.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_replace_bytes() {
        let original = b"Hello WORLD protobuf\x00\x01\x02 test WORLD end";
        let needle = b"WORLD";
        let replacement = b"UNIVERSE";
        let replaced = replace_bytes(original, needle, replacement);
        assert_eq!(
            replaced,
            b"Hello UNIVERSE protobuf\x00\x01\x02 test UNIVERSE end"
        );
    }

    #[test]
    fn test_extract_workspace_info() {
        let (name, path) = extract_workspace_info("file:///home/tiago/Proyectos/agychat%20test");
        assert_eq!(name, "agychat test");
        assert_eq!(path, "/home/tiago/Proyectos/agychat test");
    }

    #[test]
    fn test_resolve_project_and_workspace() {
        let projects = vec![crate::models::ProjectInfo {
            id: "proj-123".to_string(),
            name: "LibreMIEL".to_string(),
            folder_uri: "file:///home/tiago/Proyectos/LibreMIEL".to_string(),
        }];

        // Por project_id
        let (name, path) = resolve_project_and_workspace(
            "proj-123",
            "[\"file:///home/tiago/GoogleDrive/Universidad\"]",
            &projects,
        );
        assert_eq!(name, "LibreMIEL");
        assert_eq!(path, "/home/tiago/GoogleDrive/Universidad");

        // Por folder_uri si project_id es vacío
        let (name2, path2) = resolve_project_and_workspace(
            "",
            "file:///home/tiago/Proyectos/LibreMIEL",
            &projects,
        );
        assert_eq!(name2, "LibreMIEL");
        assert_eq!(path2, "/home/tiago/Proyectos/LibreMIEL");
    }
}
