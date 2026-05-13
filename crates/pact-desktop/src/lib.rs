use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn browse_file(app: tauri::AppHandle, mode: String) -> Result<Option<String>, String> {
    let dialog = app.dialog();
    let builder = dialog.file();

    if mode == "save" {
        let result = builder
            .set_file_name("mi_boveda.pcty")
            .add_filter("Bóveda Pact", &["pcty"])
            .blocking_save_file();
        Ok(result.map(|path| path.to_string()))
    } else {
        let result = builder
            .add_filter("Bóveda Pact", &["pcty"])
            .blocking_pick_file();
        Ok(result.map(|path| path.to_string()))
    }
}

#[derive(serde::Serialize)]
struct DerivedKeys {
    master_key: String, // Hex para JS
    auth_key: String,   // Hex para JS
}

#[tauri::command]
async fn derive_keys(password: String, salt_hex: String) -> Result<DerivedKeys, String> {
    let salt = hex::decode(salt_hex).map_err(|e| e.to_string())?;
    let (m, a) = pact_core::crypto::derivar_claves_dual(&password, &salt)
        .map_err(|e| format!("{:?}", e))?;
    
    Ok(DerivedKeys {
        master_key: hex::encode(m),
        auth_key: hex::encode(a),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![browse_file, derive_keys])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

