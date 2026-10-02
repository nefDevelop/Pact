use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

struct ServerProcess(Mutex<Option<std::process::Child>>);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        // Usamos `if let Ok` para no entrar en pánico si el mutex quedó envenenado
        // (por ejemplo, si otro hilo panicó mientras lo tenía bloqueado).
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                log::info!("Deteniendo pact-server...");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[tauri::command]
async fn browse_file(app: tauri::AppHandle, mode: String) -> Result<Option<String>, String> {
    let dialog = app.dialog();
    let builder = dialog.file();

    if mode == "save" {
        let result = builder
            .set_title("Crear Nueva Bóveda")
            .set_file_name("mi_boveda.pcty")
            .add_filter("Bóveda Pact", &["pcty"])
            .blocking_save_file();
        Ok(result.map(|path| path.to_string()))
    } else {
        let result = builder
            .set_title("Seleccionar Bóveda Existente")
            .add_filter("Bóveda Pact", &["pcty"])
            .blocking_pick_file();
        Ok(result.map(|path| path.to_string()))
    }
}

#[derive(serde::Serialize)]
struct DerivedKeys {
    master_key: String,
    auth_key: String,
    sync_key: String,
}

#[tauri::command]
async fn derive_keys(password: String, salt_hex: String) -> Result<DerivedKeys, String> {
    let t0 = std::time::Instant::now();
    let salt = hex::decode(salt_hex).map_err(|e| e.to_string())?;
    
    let (m, a) = pact_core::crypto::derivar_claves_dual(&password, &salt)
        .map_err(|e| format!("{:?}", e))?;

    // Subclave para envolver la contraseña de la BD remota (no sale del cliente
    // salvo en tránsito durante una sincronización).
    let s = pact_core::crypto::derivar_subclave(&m, b"sync");
    
    let duration = t0.elapsed();
    log::info!("[🏎️ PERFORMANCE] derive_keys (Rust/Argon2id) completado en {:?}", duration);
    
    Ok(DerivedKeys {
        master_key: hex::encode(m),
        auth_key: hex::encode(a),
        sync_key: hex::encode(s),
    })
}

/// Localiza el binario del servidor empaquetado como sidecar junto al
/// ejecutable de la app (Tauri lo copia sin el sufijo del target triple).
fn resolve_server_path() -> Option<std::path::PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for name in ["pact-server", "pact-server.exe"] {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn start_server() -> Option<std::process::Child> {
    let server_path = match resolve_server_path() {
        Some(path) => path,
        None => {
            log::warn!("pact-server (sidecar) no encontrado junto al ejecutable");
            return None;
        }
    };

    log::info!("Iniciando pact-server desde: {:?}", server_path);
    let mut cmd = std::process::Command::new(&server_path);
    // El servidor usa `info` por defecto: no fijamos RUST_LOG para que el nivel se
    // pueda elevar sin recompilar (p. ej. RUST_LOG=debug para diagnóstico).
    //
    // CORS: no fijamos `PACT_CORS_ORIGIN` porque el servidor ya permite por
    // defecto los orígenes del webview de Tauri. Fijarlo en `*` activaba CORS
    // permisivo, que dejaba leer los endpoints públicos desde cualquier página.
    cmd.env_remove("PACT_CORS_ORIGIN");
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    match cmd.spawn()
    {
        Ok(child) => {
            log::info!("pact-server iniciado con PID: {}", child.id());
            Some(child)
        }
        Err(e) => {
            log::error!("Error al iniciar pact-server: {}", e);
            None
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let app = tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![browse_file, derive_keys])
    .setup(|app| {
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .build(),
      )?;

      let child = start_server();
      app.manage(ServerProcess(Mutex::new(child)));

      Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application");

  app.run(|_handle, event| {
    if let tauri::RunEvent::Exit = event {
      log::info!("Pact Desktop finalizado.");
    }
  });
}

