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
    home_dir()
        .join(".config")
        .join("Antigravity")
        .join("logs")
        .join("language_server.log")
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
