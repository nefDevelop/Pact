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

/// Dirección de la API. Debe coincidir con el default de `pact-server`
/// (`PACT_LISTEN_ADDR`), porque el frontend llama siempre a `127.0.0.1:8002`.
fn listen_addr() -> String {
    std::env::var("PACT_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8002".to_string())
}

/// Qué hay escuchando en el puerto de la API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortProbe {
    /// No hay nada escuchando: se puede lanzar el sidecar.
    Free,
    /// Hay un `pact-server` sano.
    Pact,
    /// Hay algo más ocupando el puerto.
    Foreign,
}

/// Sondea `GET /status` con `std` para saber quién ocupa el puerto de la API.
///
/// Se hace a mano, sin cliente HTTP, para no añadir dependencias: la respuesta es
/// un JSON corto y lo único que importa es si es o no un `pact-server`.
fn probe_port(addr: &str, timeout: std::time::Duration) -> PortProbe {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};

    let socket = match addr.to_socket_addrs().ok().and_then(|mut it| it.next()) {
        Some(s) => s,
        None => return PortProbe::Foreign,
    };

    let mut stream = match TcpStream::connect_timeout(&socket, timeout) {
        Ok(s) => s,
        // Nada escuchando en este puerto.
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => return PortProbe::Free,
        Err(e) => {
            log::warn!("Sondeo de {} falló: {}", addr, e);
            return PortProbe::Foreign;
        }
    };

    if stream.set_read_timeout(Some(timeout)).is_err() {
        return PortProbe::Foreign;
    }
    let request = format!("GET /status HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return PortProbe::Foreign;
    }

    let mut buf = Vec::new();
    if stream.take(4096).read_to_end(&mut buf).is_err() {
        return PortProbe::Foreign;
    }
    let response = String::from_utf8_lossy(&buf);

    if response.starts_with("HTTP/1.1 200") && response.contains("Rust/Axum") {
        PortProbe::Pact
    } else {
        log::warn!("El puerto {} está ocupado por un servicio que no es Pact", addr);
        PortProbe::Foreign
    }
}

/// Aviso que debe ver el usuario, si el arranque no fue el esperado.
fn notify_user(app: &tauri::AppHandle, title: &str, message: &str, error: bool) {
    // El closure de `run_on_main_thread` debe ser `'static`, así que el handle y
    // las cadenas se copian en lugar de capturarse por referencia.
    let handle = app.clone();
    let dialog_handle = handle.clone();
    let (title, message) = (title.to_string(), message.to_string());
    let _ = handle.run_on_main_thread(move || {
        let dialog = dialog_handle.dialog().message(&message).title(&title);
        if error {
            dialog
                .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                .blocking_show();
        } else {
            dialog.blocking_show();
        }
    });
}

/// Avisa si, tras arrancar, la API no llega a responder en `timeout`.
fn watch_server_startup(app: tauri::AppHandle, addr: String, timeout: std::time::Duration) {
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if probe_port(&addr, std::time::Duration::from_millis(300)) == PortProbe::Pact {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let msg = format!(
            "El servidor de Pact no respondió en {addr}. Revisa el terminal para ver el error."
        );
        log::error!("{}", msg);
        notify_user(&app, "Pact no pudo arrancar", &msg, true);
    });
}

/// Lanza el sidecar, salvo que el puerto ya esté ocupado.
fn start_server(
    addr: &str,
) -> (
    Option<std::process::Child>,
    Option<(String, String, bool)>,
) {
    match probe_port(addr, std::time::Duration::from_millis(500)) {
        PortProbe::Pact => {
            log::info!("Ya hay un pact-server en {}: se reutiliza", addr);
            return (
                None,
                Some((
                    "Pact ya está en marcha".into(),
                    format!(
                        "Hay otra instancia de Pact escuchando en {addr}. \
                         Se reutiliza su servidor: no abras la misma bóveda \
                         en dos ventanas a la vez."
                    ),
                    false,
                )),
            );
        }
        PortProbe::Foreign => {
            log::error!("El puerto {} está ocupado por otro servicio", addr);
            return (
                None,
                Some((
                    "Puerto ocupado".into(),
                    format!(
                        "El puerto {addr} ya lo usa otro servicio, así que el \
                         servidor no puede arrancar. Libera el puerto o define \
                         PACT_LISTEN_ADDR con otro valor."
                    ),
                    true,
                )),
            );
        }
        PortProbe::Free => {}
    }

    let server_path = match resolve_server_path() {
        Some(path) => path,
        None => {
            log::warn!("pact-server (sidecar) no encontrado junto al ejecutable");
            return (
                None,
                Some((
                    "Falta el servidor".into(),
                    "No se encontró pact-server junto al ejecutable.".into(),
                    true,
                )),
            );
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
            (Some(child), None)
        }
        Err(e) => {
            log::error!("Error al iniciar pact-server: {}", e);
            (
                None,
                Some((
                    "No se pudo iniciar el servidor".into(),
                    format!("No se pudo lanzar pact-server: {e}"),
                    true,
                )),
            )
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let addr = listen_addr();

  let app = tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![browse_file, derive_keys])
    .setup(move |app| {
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .build(),
      )?;

      let (child, notice) = start_server(&addr);
      app.manage(ServerProcess(Mutex::new(child)));

      // Si el servidor arrancó, confirmamos que llega a responder; si no,
      // avisamos en vez de dejar una ventana que falla en silencio.
      if notice.is_none() {
        watch_server_startup(
          app.handle().clone(),
          addr.clone(),
          std::time::Duration::from_secs(5),
        );
      }

      if let Some((title, message, error)) = notice {
        log::error!("Arranque con incidencias: {}", message);
        notify_user(app.handle(), &title, &message, error);
      }

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

#[cfg(test)]
mod tests {
    use super::{probe_port, PortProbe};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    fn free_addr() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener);
        addr
    }

    /// Servidor de mentira que responde siempre lo mismo en cada conexión.
    fn serve(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                std::thread::spawn(move || {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);
                    let _ = stream.write_all(response.as_bytes());
                });
            }
        });
        addr
    }

    #[test]
    fn puerto_libre_se_detecta_como_libre() {
        assert_eq!(probe_port(&free_addr(), Duration::from_millis(500)), PortProbe::Free);
    }

    #[test]
    fn pact_server_se_reconoce() {
        let addr = serve(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n\
             {\"status\":\"Running\",\"engine\":\"Rust/Axum\"}\r\n",
        );
        assert_eq!(probe_port(&addr, Duration::from_millis(1000)), PortProbe::Pact);
    }

    #[test]
    fn servicio_ajeno_se_detecta_como_ajeno() {
        let addr = serve("HTTP/1.1 200 OK\r\n\r\n{\"server\":\"nginx\"}\r\n");
        assert_eq!(probe_port(&addr, Duration::from_millis(1000)), PortProbe::Foreign);
    }

    /// Un servicio que acepta la conexión pero no habla HTTP: leer la respuesta
    /// falla, y aun así hay que reportarlo como ocupado, no como libre.
    #[test]
    fn servicio_sin_http_no_se_toma_por_libre() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                // Acepta y se queda callado: el sondeo no puede leer nada.
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(400));
                    let _ = stream.write_all(b"");
                });
            }
        });
        assert_eq!(probe_port(&addr, Duration::from_millis(500)), PortProbe::Foreign);
    }
}

