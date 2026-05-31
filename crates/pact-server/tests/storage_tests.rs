use std::fs::File;
use std::io::{Write, Read};
use std::path::PathBuf;
use tempfile::tempdir;
use pact_server::storage::StorageManager;
use zip::{ZipArchive, ZipWriter, write::FileOptions};

#[test]
fn test_vault_initialization_and_validation() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.pcty");
    let storage = StorageManager::new(None);

    // 1. Validar que un archivo inexistente no es válido
    assert!(!storage.validate_vault(&vault_path).unwrap());

    // 2. Inicializar y validar
    storage.init_vault(&vault_path).unwrap();
    assert!(vault_path.exists());
    assert!(storage.validate_vault(&vault_path).unwrap());

    // 3. Modificar el archivo para que sea inválido (firma incorrecta)
    {
        let file = File::create(&vault_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        zip.start_file("pact.sig", options).unwrap();
        zip.write_all(b"Wrong Signature").unwrap();
        zip.finish().unwrap();
    }
    assert!(!storage.validate_vault(&vault_path).unwrap());
}

#[test]
fn test_save_and_extract_db_integrity() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("integrity.pcty");
    let storage = StorageManager::new(Some(vault_path.clone()));
    storage.init_vault(&vault_path).unwrap();

    let username = "alice";
    let db_content = b"fake-sqlite-binary-data-for-alice";
    let local_db_path = dir.path().join("alice_local.db");
    
    {
        let mut f = File::create(&local_db_path).unwrap();
        f.write_all(db_content).unwrap();
    }

    // 1. Guardar
    storage.save_db(username, &local_db_path).unwrap();

    // 2. Extraer y comparar
    let extracted_path = dir.path().join("alice_extracted.db");
    let found = storage.extract_db(username, &extracted_path).unwrap();
    assert!(found);

    let mut extracted_content = Vec::new();
    File::open(&extracted_path).unwrap().read_to_end(&mut extracted_content).unwrap();
    assert_eq!(extracted_content, db_content);
}

#[test]
fn test_list_users_multiple() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("multi_users.pcty");
    let storage = StorageManager::new(Some(vault_path.clone()));
    storage.init_vault(&vault_path).unwrap();

    let users = vec!["bob", "charlie", "dave"];
    for user in &users {
        let db_path = dir.path().join(format!("{}.db", user));
        File::create(&db_path).unwrap().write_all(user.as_bytes()).unwrap();
        storage.save_db(user, &db_path).unwrap();
    }

    let listed_users = storage.list_users().unwrap();
    assert_eq!(listed_users.len(), 3);
    for user in users {
        assert!(listed_users.contains(&user.to_string()));
    }
}

#[test]
fn test_malicious_zip_slip_prevention_logic() {
    // Aunque StorageManager usa nombres de archivo controlados (username + .db),
    // es importante verificar que no podemos "engañarlo" si el zip tuviera entradas extrañas.
    // Nota: StorageManager::extract_db busca específicamente por "username.db"
    
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("evil.pcty");
    
    {
        let file = File::create(&vault_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        
        // Intentar un zip slip en una entrada que no debería ser extraída
        zip.start_file("../../../etc/passwd.db", options).unwrap();
        zip.write_all(b"evil").unwrap();
        zip.finish().unwrap();
    }

    let storage = StorageManager::new(Some(vault_path.clone()));
    
    // 1. Extraer un usuario legítimo no debería encontrar nada
    let dest = dir.path().join("safe.db");
    let found = storage.extract_db("alice", &dest).unwrap();
    assert!(!found);

    // 2. Listar usuarios no debería incluir la ruta maliciosa como un usuario válido (opcional, depende de list_users)
    let users = storage.list_users().unwrap();
    assert!(!users.contains(&"../../../etc/passwd".to_string()));
}

#[test]
fn test_storage_concurrency_stress() {
    use std::sync::Arc;
    use std::thread;

    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("stress.pcty");
    let storage = Arc::new(StorageManager::new(Some(vault_path.clone())));
    storage.init_vault(&vault_path).unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let storage_clone = storage.clone();
        let thread_dir = dir.path().to_path_buf();
        handles.push(thread::spawn(move || {
            let user = format!("user_{}", i);
            let db_path = thread_dir.join(format!("{}.db", user));
            File::create(&db_path).unwrap().write_all(user.as_bytes()).unwrap();
            
            for _ in 0..5 {
                storage_clone.save_db(&user, &db_path).unwrap();
                let mut dest = Vec::new();
                let tmp_dest = thread_dir.join(format!("{}_tmp.db", user));
                storage_clone.extract_db(&user, &tmp_dest).unwrap();
                File::open(&tmp_dest).unwrap().read_to_end(&mut dest).unwrap();
                assert_eq!(dest, user.as_bytes());
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_users = storage.list_users().unwrap();
    assert_eq!(final_users.len(), 10);
}
