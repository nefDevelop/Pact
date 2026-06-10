use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zip::{ZipArchive, ZipWriter, write::FileOptions};
use tempfile::NamedTempFile;

use std::sync::Mutex;

pub struct StorageManager {
    pcty_path: Mutex<Option<PathBuf>>,
}

impl StorageManager {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { pcty_path: Mutex::new(path) }
    }

    pub fn set_path(&self, path: Option<PathBuf>) {
        if let Ok(mut p) = self.pcty_path.lock() {
            println!("[📦 STORAGE] Actualizando ruta de bóveda: {:?}", path);
            *p = path;
        }
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.pcty_path.lock().ok()?.clone()
    }

    /// Inicializa una nueva bóveda con la firma pact.sig
    pub fn init_vault(&self, path: &Path) -> io::Result<()> {
        println!("[📦 STORAGE] Inicializando nueva bóveda en {:?}", path);
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        
        let options = FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        
        zip.start_file("pact.sig", options)?;
        zip.write_all(b"Pact Vault v1.0")?;
        zip.finish()?;
        
        Ok(())
    }

    /// Valida que el archivo en la ruta dada sea una bóveda de Pact válida
    pub fn validate_vault(&self, path: &Path) -> io::Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        let file = File::open(path)?;
        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(_) => return Ok(false), // No es un zip válido
        };

        let mut sig_file = match archive.by_name("pact.sig") {
            Ok(f) => f,
            Err(_) => return Ok(false), // No tiene la firma
        };

        let mut sig_content = String::new();
        use std::io::Read;
        sig_file.read_to_string(&mut sig_content)?;

        Ok(sig_content == "Pact Vault v1.0")
    }

    /// Guarda la base de datos de un usuario en el contenedor .pcty
    pub fn save_db(&self, username: &str, db_path: &Path) -> io::Result<()> {
        let t_total = std::time::Instant::now();
        let path_lock = self.pcty_path.lock().unwrap();
        let pcty_path = path_lock.as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No vault linked"))?;

        println!("[📦 STORAGE] Guardando base de datos de {} en {:?}", username, pcty_path);

        let db_name = format!("{}.db", username);
        let parent_dir = pcty_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let temp_pcty = NamedTempFile::new_in(parent_dir)?;
        
        {
            let mut zin = ZipArchive::new(File::open(pcty_path)?)?;
            let mut zout = ZipWriter::new(&temp_pcty);

            // Copiar archivos existentes excepto el que vamos a actualizar
            for i in 0..zin.len() {
                let file = zin.by_index(i)?;
                if file.name() != db_name {
                    zout.raw_copy_file(file)?;
                }
            }

            // Añadir el nuevo archivo DB
            let options = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated);
            
            zout.start_file(db_name, options)?;
            let mut db_file = File::open(db_path)?;
            io::copy(&mut db_file, &mut zout)?;
        }

        // Reemplazar el original con el temporal
        temp_pcty.persist(pcty_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
        println!("[🏎️ PERFORMANCE] save_db (Zip I/O) completado en {:?}", t_total.elapsed());
        Ok(())
    }

    /// Extrae la base de datos de un usuario del contenedor .pcty
    pub fn extract_db(&self, username: &str, dest_path: &Path) -> io::Result<bool> {
        let t_total = std::time::Instant::now();
        let path_lock = self.pcty_path.lock().unwrap();
        let pcty_path = path_lock.as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No vault linked"))?;

        println!("[📦 STORAGE] Extrayendo base de datos de {} desde {:?}", username, pcty_path);

        let db_name = format!("{}.db", username);
        let mut zip = ZipArchive::new(File::open(pcty_path)?)?;
        
        // Búsqueda case-insensitive (como en el original)
        let mut real_name = None;
        for i in 0..zip.len() {
            let file = zip.by_index(i)?;
            if file.name().to_lowercase() == db_name.to_lowercase() {
                real_name = Some(file.name().to_string());
                break;
            }
        }

        if let Some(name) = real_name {
            let mut file = zip.by_name(&name)?;
            let mut dest = File::create(dest_path)?;
            io::copy(&mut file, &mut dest)?;
            println!("[🏎️ PERFORMANCE] extract_db (Zip I/O) completado en {:?}", t_total.elapsed());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Lista los usuarios disponibles en la bóveda
    pub fn list_users(&self) -> io::Result<Vec<String>> {
        let path_lock = self.pcty_path.lock().unwrap();
        let pcty_path = path_lock.as_ref();
        
        if pcty_path.is_none() || !pcty_path.unwrap().exists() {
            return Ok(vec![]);
        }

        let mut zip = ZipArchive::new(File::open(pcty_path.unwrap())?)?;
        let mut users = Vec::new();
        
        tracing::debug!("[STORAGE] list_users: Analizando {} archivos en el .pcty", zip.len());
        for i in 0..zip.len() {
            let file = zip.by_index(i)?;
            let name = file.name();
            tracing::debug!("[STORAGE] list_users: Encontrado archivo '{}'", name);
            
            // Validación de seguridad: 
            // 1. Debe terminar en .db
            // 2. No debe contener caracteres de ruta (evitar Zip Slip o listados maliciosos)
            // 3. Debe ser un nombre de archivo simple
            if name.to_lowercase().ends_with(".db") && !name.contains('/') && !name.contains('\\') {
                // Quitamos los últimos 3 caracteres (".db") de forma segura
                let user_name = &name[..name.len() - 3];
                users.push(user_name.to_string());
            }
        }
        
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_vault_lifecycle() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("test.pcty");
        let storage = StorageManager::new(Some(vault_path.clone()));

        // 1. Inicializar
        storage.init_vault(&vault_path).unwrap();
        assert!(vault_path.exists());

        // 2. Verificar firma
        let file = File::open(&vault_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut sig_file = archive.by_name("pact.sig").unwrap();
        let mut sig_content = String::new();
        sig_file.read_to_string(&mut sig_content).unwrap();
        assert_eq!(sig_content, "Pact Vault v1.0");

        // 3. Guardar DB ficticia
        let db_path = dir.path().join("user1.db");
        {
            let mut f = File::create(&db_path).unwrap();
            f.write_all(b"sqlite-data").unwrap();
        }
        storage.save_db("user1", &db_path).unwrap();

        // 4. Listar usuarios
        let users = storage.list_users().unwrap();
        assert_eq!(users, vec!["user1"]);

        // 5. Extraer DB
        let extracted_path = dir.path().join("user1_restored.db");
        let found = storage.extract_db("user1", &extracted_path).unwrap();
        assert!(found);
        
        let mut extracted_content = String::new();
        File::open(&extracted_path).unwrap().read_to_string(&mut extracted_content).unwrap();
        assert_eq!(extracted_content, "sqlite-data");
    }

    #[test]
    fn test_vault_validation() {
        let dir = tempdir().unwrap();
        let storage = StorageManager::new(None);

        // 1. No existe
        let path = dir.path().join("missing.pcty");
        assert!(!storage.validate_vault(&path).unwrap());

        // 2. Firma correcta
        let path = dir.path().join("valid.pcty");
        storage.init_vault(&path).unwrap();
        assert!(storage.validate_vault(&path).unwrap());

        // 3. Sin firma (zip vacío)
        let path = dir.path().join("no_sig.pcty");
        {
            let file = File::create(&path).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.finish().unwrap();
        }
        assert!(!storage.validate_vault(&path).unwrap());

        // 4. Firma incorrecta
        let path = dir.path().join("bad_sig.pcty");
        {
            let file = File::create(&path).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.start_file("pact.sig", FileOptions::<()>::default()).unwrap();
            zip.write_all(b"Mala firma").unwrap();
            zip.finish().unwrap();
        }
        assert!(!storage.validate_vault(&path).unwrap());

        // 5. Archivo corrupto (no es un zip)
        let path = dir.path().join("corrupt.pcty");
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"not a zip file").unwrap();
        }
        assert!(!storage.validate_vault(&path).unwrap());
    }

    #[test]
    fn test_storage_path_management() {
        let storage = StorageManager::new(None);
        assert!(storage.get_path().is_none());

        let path = PathBuf::from("/tmp/test.pcty");
        storage.set_path(Some(path.clone()));
        assert_eq!(storage.get_path(), Some(path));
    }
}
