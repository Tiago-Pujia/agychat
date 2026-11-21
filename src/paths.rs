use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| {
        if let Ok(user) = std::env::var("USER") {
            PathBuf::from(format!("/home/{}", user))
        } else {
            PathBuf::from(".")
        }
    })
}

pub fn base_dir() -> PathBuf {
    home_dir().join(".gemini").join("antigravity")
}

pub fn summaries_db_path() -> PathBuf {
    base_dir().join("conversation_summaries.db")
}

pub fn conversations_dir() -> PathBuf {
    base_dir().join("conversations")
}

pub fn annotations_dir() -> PathBuf {
    base_dir().join("annotations")
}

pub fn brain_dir() -> PathBuf {
    base_dir().join("brain")
}

pub fn ls_log_path() -> PathBuf {
    let mut candidates = Vec::new();

    if let Some(config) = dirs::config_dir() {
        candidates.push(config.join("Antigravity").join("logs").join("language_server.log"));
    }

    let home = home_dir();
    candidates.push(home.join(".config").join("Antigravity").join("logs").join("language_server.log"));

    #[cfg(target_os = "macos")]
    {
        candidates.push(home.join("Library").join("Application Support").join("Antigravity").join("logs").join("language_server.log"));
        candidates.push(home.join("Library").join("Logs").join("Antigravity").join("language_server.log"));
    }

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    if let Some(config) = dirs::config_dir() {
        config.join("Antigravity").join("logs").join("language_server.log")
    } else {
        home.join(".config").join("Antigravity").join("logs").join("language_server.log")
    }
}

pub fn projects_dir() -> PathBuf {
    home_dir()
        .join(".gemini")
        .join("config")
        .join("projects")
}

pub fn agyhub_summaries_pb_path() -> PathBuf {
    base_dir().join("agyhub_summaries_proto.pb")
}
