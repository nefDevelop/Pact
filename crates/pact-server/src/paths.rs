//! Resolución de rutas de datos de Pact.
//!
//! Estas rutas las usan el arranque del servidor (`main.rs`), el gestor de
//! bóveda (`storage`) y el seed de demo, así que viven aquí en vez de duplicarse.
//!
//! Todo sale de `PACT_DATA_DIR` y, si no está, del directorio de datos del
//! usuario (XDG en Linux, `%APPDATA%` en Windows, `~/Library/Application
//! Support` en macOS). El directorio de trabajo solo se usa como reserva porque
//! en un AppImage es el montaje squashfs de solo lectura.

use std::path::{Path, PathBuf};

/// Directorio de datos persistente de Pact, o `None` si no se pudo determinar.
pub fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("PACT_DATA_DIR") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }

    let base: PathBuf = if cfg!(target_os = "windows") {
        match std::env::var("APPDATA") {
            Ok(a) => PathBuf::from(a),
            Err(_) => return None,
        }
    } else if cfg!(target_os = "macos") {
        match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h).join("Library/Application Support"),
            Err(_) => return None,
        }
    } else {
        match std::env::var("XDG_DATA_HOME") {
            Ok(x) => PathBuf::from(x),
            Err(_) => match std::env::var("HOME") {
                Ok(h) => PathBuf::from(h).join(".local/share"),
                Err(_) => return None,
            },
        }
    };

    let dir = base.join("pact");
    match std::fs::create_dir_all(&dir) {
        Ok(()) => Some(dir),
        Err(e) => {
            tracing::warn!("No se pudo crear el directorio de datos ({}): {}", dir.display(), e);
            None
        }
    }
}

/// Directorio de datos con reserva: si no se puede determinar (ni por variable
/// ni por el directorio del usuario), cae al actual en vez de fallar.
pub fn data_dir_or_cwd() -> PathBuf {
    if let Some(dir) = data_dir() {
        return dir;
    }
    tracing::warn!(
        "No se pudo determinar el directorio de datos; se usa el actual. \
         Define PACT_DATA_DIR si el proceso no puede escribir en él."
    );
    PathBuf::from(".")
}

/// Ruta del archivo SQLite de la bóveda, resolviendo `DATABASE_URL`.
///
/// El URL solo admite `sqlite:` (MySQL/MariaDB es destino de sincronización
/// remota, no base local). Se devuelve sin el prefijo para poder usarla como
/// `Path`: el gestor de bóveda la necesita para extraer y guardar la base del
/// usuario dentro del `.pcty`, y antes usaba `Path::new("pact.db")`, que con un
/// `DATABASE_URL` distinto escribía en el directorio de trabajo.
pub fn resolve_db_path(default_dir: &Path) -> PathBuf {
    match std::env::var("DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => {
            let raw = url.trim();
            match raw.strip_prefix("sqlite:") {
                Some(rest) => PathBuf::from(rest),
                // El `main` ya aborta en este caso; aquí solo se avisa para que
                // un fallo no se manifieste como un `pact.db` en el CWD.
                None => {
                    tracing::error!(
                        "DATABASE_URL solo admite SQLite (p. ej. 'sqlite:pact.db'); recibido '{}'",
                        raw
                    );
                    PathBuf::from("pact.db")
                }
            }
        }
        _ => default_dir.join("pact.db"),
    }
}