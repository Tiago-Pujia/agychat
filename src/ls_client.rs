use crate::models::LiveLsInfo;
use crate::paths::ls_log_path;
use regex::Regex;
use serde_json::json;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::time::Duration;

/// Detecta si language_server está corriendo y obtiene su puerto HTTPS y CSRF token.
pub fn get_live_ls_info() -> Option<LiveLsInfo> {
    let mut csrf_token = None;

    // 1. Escanear /proc buscando el proceso language_server y el parámetro --csrf_token (Linux)
    #[cfg(target_os = "linux")]
    if let Ok(entries) = fs::read_dir("/proc") {
        let re_null = Regex::new(r"--csrf_token\x00([a-f0-9-]+)").ok()?;
        let re_space = Regex::new(r"--csrf_token\s+([a-f0-9-]+)").ok()?;

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let pid_str = file_name.to_string_lossy();
            if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let cmdline_path = entry.path().join("cmdline");
            if let Ok(bytes) = fs::read(&cmdline_path) {
                let cmdline = String::from_utf8_lossy(&bytes);
                if cmdline.contains("language_server") && cmdline.contains("--csrf_token") {
                    if let Some(caps) = re_null.captures(&cmdline) {
                        csrf_token = Some(caps[1].to_string());
                        break;
                    } else if let Some(caps) = re_space.captures(&cmdline) {
                        csrf_token = Some(caps[1].to_string());
                        break;
                    }
                }
            }
        }
    }

    // Si no se detectó por /proc (macOS, Windows o entorno sin /proc), usar sysinfo
    if csrf_token.is_none() {
        use sysinfo::{ProcessesToUpdate, System};
        if let Ok(mut sys) = System::new() {
            sys.refresh_processes(ProcessesToUpdate::All, true);
            if let Ok(re_token) = Regex::new(r"--csrf_token(?:=|\s+|\x00)([a-f0-9-]+)") {
                for process in sys.processes().values() {
                    let name = process.name().to_string_lossy();
                    let cmd: Vec<String> = process
                        .cmd()
                        .iter()
                        .map(|s| s.to_string_lossy().into_owned())
                        .collect();
                    let cmd_str = cmd.join(" ");
                    if (name.contains("language_server") || cmd_str.contains("language_server"))
                        && cmd_str.contains("--csrf_token")
                    {
                        if let Some(caps) = re_token.captures(&cmd_str) {
                            csrf_token = Some(caps[1].to_string());
                            break;
                        }
                    }
                }
            }
        }
    }

    let csrf = csrf_token?;

    // 2. Leer language_server.log para encontrar el puerto HTTPS asignado
    let log_path = ls_log_path();
    if !log_path.exists() {
        return None;
    }

    let file = File::open(&log_path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let re_port = Regex::new(r"listening on random port at (\d+) for HTTPS").ok()?;
    let mut port = None;

    for line in lines.iter().rev() {
        if let Some(caps) = re_port.captures(line) {
            if let Ok(p) = caps[1].parse::<u16>() {
                port = Some(p);
                break;
            }
        }
    }

    port.map(|p| LiveLsInfo {
        port: p,
        csrf_token: csrf,
    })
}

/// Construye un cliente HTTP configurado para aceptar certificados autofirmados locales de Antigravity.
fn create_client() -> Result<reqwest::blocking::Client, reqwest::Error> {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(10))
        .build()
}

/// Invoca los endpoints Connect/gRPC en vivo de Antigravity para bifurcar (fork) un chat.
#[allow(dead_code)]
pub fn fork_via_api(
    source_id: &str,
    new_title: &str,
    port: u16,
    csrf_token: &str,
    step_count: i64,
) -> Result<String, String> {
    let client = create_client().map_err(|e| format!("Error creando cliente HTTP: {}", e))?;
    let fork_url = format!(
        "https://127.0.0.1:{}/exa.language_server_pb.LanguageServerService/ForkConversation",
        port
    );

    let fork_data = json!({
        "source_cascade_id": source_id,
        "fork_at_step_index": std::cmp::max(0, step_count - 1),
    });

    let resp = client
        .post(&fork_url)
        .header("Content-Type", "application/json")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&fork_data)
        .send()
        .map_err(|e| format!("Fallo al enviar solicitud ForkConversation: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "ForkConversation respondió con código de estado HTTP {}",
            resp.status()
        ));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Error decodificando respuesta JSON: {}", e))?;

    let new_cascade_id = resp_json
        .get("newCascadeId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("No se recibió newCascadeId en la respuesta: {}", resp_json))?
        .to_string();

    // Actualizar anotaciones del nuevo chat con su título asignado
    update_annotations_via_api(&new_cascade_id, new_title, port, csrf_token);

    Ok(new_cascade_id)
}

/// Actualiza anotaciones de título y refresca la interfaz gráfica de Antigravity en vivo.
pub fn update_annotations_via_api(
    cascade_id: &str,
    title: &str,
    port: u16,
    csrf_token: &str,
) -> bool {
    let client = match create_client() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!(
        "https://127.0.0.1:{}/exa.language_server_pb.LanguageServerService/UpdateConversationAnnotations",
        port
    );

    let data = json!({
        "cascade_id": cascade_id,
        "cascade_ids": [cascade_id],
        "annotations": { "title": title },
        "merge_annotations": true,
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&data)
        .send();

    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// Solicita a language_server cargar una trayectoria de conversación en memoria.
pub fn load_trajectory_via_api(cascade_id: &str, port: u16, csrf_token: &str) -> bool {
    let client = match create_client() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!(
        "https://127.0.0.1:{}/exa.language_server_pb.LanguageServerService/LoadTrajectory",
        port
    );

    let data = json!({
        "cascade_id": cascade_id
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&data)
        .send();

    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// Notifica a language_server para eliminar y descargar la conversación en memoria.
pub fn delete_via_api(cascade_id: &str, port: u16, csrf_token: &str) -> bool {
    let client = match create_client() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!(
        "https://127.0.0.1:{}/exa.language_server_pb.LanguageServerService/DeleteCascadeTrajectory",
        port
    );

    let data = json!({
        "cascade_id": cascade_id
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&data)
        .send();

    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}
