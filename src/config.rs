use std::path::PathBuf;

const APP: &str = "cull";

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP)
        .join("config.json")
}

pub fn load_dest() -> Option<PathBuf> {
    let text = std::fs::read_to_string(config_path()).ok()?;
    let val: serde_json::Value = serde_json::from_str(&text).ok()?;
    val["dest"].as_str().map(PathBuf::from)
}

pub fn save_dest(path: &PathBuf) {
    let p = config_path();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::json!({ "dest": path.to_string_lossy() });
    let _ = std::fs::write(&p, json.to_string());
}
