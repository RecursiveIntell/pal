use serde_json::Value;
use std::fs;
use std::path::Path;

fn templates_dir() -> String {
    format!("{}/../src/public/templates", env!("CARGO_MANIFEST_DIR"))
}

#[tauri::command]
pub async fn list_templates() -> Result<Vec<String>, String> {
    let base = templates_dir();
    let path = Path::new(&base);
    let mut files = Vec::new();
    if path.exists() {
        let rd = fs::read_dir(path).map_err(|e| e.to_string())?;
        for item in rd.flatten() {
            if let Some(name) = item.file_name().to_str() {
                if name.ends_with(".json") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

#[tauri::command]
pub async fn render_template(template_name: String, params: Value) -> Result<String, String> {
    let path = format!("{}/{template_name}", templates_dir());
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut rendered = raw;
    if let Some(obj) = params.as_object() {
        for (k, v) in obj {
            let token = format!("{{{{{k}}}}}");
            let replacement = v.to_string();
            rendered = rendered.replace(&token, replacement.trim_matches('"'));
        }
    }
    Ok(rendered)
}
