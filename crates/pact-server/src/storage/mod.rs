pub mod zip;

pub use zip::StorageManager;

use std::path::{Path, PathBuf};

/// Archivo donde se recuerda la última bóveda elegida por el usuario.
fn remembered_vault_file(data_dir: &Path) -> PathBuf {
    data_dir.join("vault_path")
}

/// Lee la bóveda recordada de la última vez. `None` si no hay o está vacía.
pub fn read_remembered_vault_path(data_dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(remembered_vault_file(data_dir)).ok()?;
    let path = raw.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Recuerda la bóveda elegida entre reinicios. `None` la olvida.
///
/// Es un archivo de texto y no una tabla porque la ruta hace falta antes de
/// abrir la base de datos, y es lo único que sobrevive a un `reset()` de la BD.
pub fn remember_vault_path(data_dir: &Path, path: Option<&Path>) -> std::io::Result<()> {
    let file = remembered_vault_file(data_dir);
    match path {
        Some(p) => {
            std::fs::write(&file, p.to_string_lossy().as_bytes())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
            }
            Ok(())
        }
        None => match std::fs::remove_file(&file) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        },
    }
}
