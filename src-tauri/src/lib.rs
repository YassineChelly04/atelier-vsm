//! Atelier VSM — coquille desktop Tauri.
//!
//! Le front (HTML/SVG/JS) gère l'édition. Le backend Rust fournit
//! l'accès natif aux fichiers et un moteur de calcul VSM indépendant
//! (utilisé pour le rapport CSV et testé unitairement).

pub mod vsm;

use std::fs;

#[tauri::command]
fn save_text(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| format!("{path} : {e}"))
}

#[tauri::command]
fn load_text(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("{path} : {e}"))
}

#[tauri::command]
fn report_csv(doc: String) -> Result<String, String> {
    let d: vsm::Doc = serde_json::from_str(&doc).map_err(|e| e.to_string())?;
    Ok(vsm::report_csv(&d))
}

#[tauri::command]
fn page_metrics(page: String) -> Result<vsm::Metrics, String> {
    let p: vsm::Page = serde_json::from_str(&page).map_err(|e| e.to_string())?;
    Ok(vsm::metrics(&p))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![save_text, load_text, report_csv, page_metrics])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'Atelier VSM");
}
