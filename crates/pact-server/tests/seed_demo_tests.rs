//! Tests del seed de demo (`src/bin/seed_demo.rs`).
//!
//! El seed cifra el contenido con la misma clave que el navegador, así que lo
//! relevante no es solo que haya filas: es que lo sembrado se pueda *descifrar*
//! con la contraseña que se documenta en el README. Si el seed se desincroniza
//! de `pact-core` o de la derivación del servidor, la demo abriría con la lista
//! de entradas ilegibles, y eso solo se detecta descifrando.

use std::fs::File;
use std::path::Path;
use std::process::Command;

use pact_core::crypto::{abrir_dato_json, derivar_claves_dual, derivar_masterkey};
use pact_server::storage::StorageManager;
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::tempdir;

const DEMO_PASSWORD: &str = "DemoPass123!";
const DEMO_SALT: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90";

/// La sal como la usan los clientes (navegador vía WASM y escritorio vía
/// comando Tauri): 16 bytes crudos decodificados del hex, no el ASCII.
fn salt_cruda() -> Vec<u8> {
    hex::decode(DEMO_SALT).expect("la sal de la demo debe ser hex válido")
}

/// Ejecuta el binario del seed contra un directorio temporal.
fn seed(data_dir: &Path) {
    let salida = Command::new(env!("CARGO_BIN_EXE_seed_demo"))
        .args(["--data-dir", data_dir.to_str().unwrap(), "--force"])
        .output()
        .expect("ejecutando seed_demo");
    assert!(
        salida.status.success(),
        "seed_demo falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
}

async fn pool(data_dir: &Path) -> sqlx::SqlitePool {
    let ruta = data_dir.join("pact.db").canonicalize().unwrap();
    // sqlx solo abre; el archivo lo crea el seed.
    assert!(ruta.exists(), "el seed no creó {}", ruta.display());
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{}", ruta.display()))
        .await
        .unwrap()
}

#[tokio::test]
async fn seed_demo_pobla_la_boveda() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("demo");
    seed(&data_dir);

    let db = pool(&data_dir).await;

    let contar = async |tabla: &str| -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {}", tabla);
        sqlx::query_scalar::<_, i64>(&sql)
            .fetch_one(&db)
            .await
            .unwrap()
    };

    assert_eq!(contar("users").await, 1, "un solo usuario demo");
    assert_eq!(contar("folders").await, 5);
    // 18 cuentas + 3 en la papelera (soft-delete).
    assert_eq!(contar("accounts").await, 21);
    let vivas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE deleted_at IS NULL")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(vivas, 18, "la papelera no debe contar como cuentas vivas");
    assert_eq!(contar("totp_settings").await, 3);

    // Favoritos: la captura de la vista de favoritos necesita alguno.
    let favoritos: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE is_favorite = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(favoritos >= 3, "favoritos = {}", favoritos);

    // Cada cuenta tiene que estar en una carpeta viva.
    let huerfanas: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE folder_id IS NULL AND deleted_at IS NULL")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(huerfanas, 0, "hay cuentas vivas sin carpeta");
}

#[tokio::test]
async fn seed_demo_cifra_todo_el_contenido() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("demo");
    seed(&data_dir);

    let db = pool(&data_dir).await;
    let (master_key, _) = derivar_claves_dual(DEMO_PASSWORD, &salt_cruda()).unwrap();

    // Si el seed guardara algo en claro, la UI lo mostraría sin descifrar.
    let filas: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT title, url, username_site, encrypted_pass FROM accounts LIMIT 5",
    )
    .fetch_all(&db)
    .await
    .unwrap();

    assert_eq!(filas.len(), 5);
    for (titulo, url, usuario, pass) in &filas {
        for cifrado in [titulo, url, usuario, pass] {
            assert!(
                cifrado.starts_with('{') && cifrado.contains("\"nonce\""),
                "campo sin cifrar: {}",
                cifrado
            );
            let claro = abrir_dato_json(cifrado, &master_key)
                .unwrap_or_else(|e| panic!("no se pudo descifrar ({}): {:?}", cifrado, e));
            let valor: String =
                serde_json::from_slice(&claro).expect("el plano debe ser JSON de una cadena");
            assert!(!valor.is_empty());
        }
    }

    // Y el título descifrado tiene que ser uno de los del seed: si el formato
    // del blob cambiara, `abrir_dato_json` devolvería basura en vez de fallar.
    let claro = abrir_dato_json(&filas[0].0, &master_key).unwrap();
    let titulo: String = serde_json::from_slice(&claro).unwrap();
    assert!(
        ["Gmail", "GitHub", "Netflix", "Spotify", "Amazon"].contains(&titulo.as_str()),
        "título inesperado tras descifrar: {}",
        titulo
    );

    // La semilla TOTP también va cifrada.
    let semilla: String =
        sqlx::query_scalar("SELECT encrypted_seed FROM totp_settings LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
    let plano = abrir_dato_json(&semilla, &master_key).unwrap();
    let valor: String = serde_json::from_slice(&plano).unwrap();
    assert_eq!(valor.len(), 16, "la semilla TOTP son 16 caracteres base32");
}

#[tokio::test]
async fn seed_demo_la_contrasena_documentada_sirve() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("demo");
    seed(&data_dir);

    let db = pool(&data_dir).await;
    let (master_hash, salt): (String, String) =
        sqlx::query_as("SELECT master_hash, salt FROM users WHERE username = 'demo'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(salt, DEMO_SALT);

    // Esto es exactamente lo que hace el servidor al autenticar: el cliente
    // (navegador o escritorio) deriva la authKey sobre los 16 bytes crudos de
    // la sal y manda su hex; el servidor deriva sobre ese string con los bytes
    // ASCII de la sal. Si el seed derivara sobre otra cosa, el usuario de la
    // demo no podría desbloquear la bóveda.
    let (_, auth_key) = derivar_claves_dual(DEMO_PASSWORD, &salt_cruda()).unwrap();
    let esperado = derivar_masterkey(&hex::encode(&auth_key), salt.as_bytes()).unwrap();
    assert_eq!(
        master_hash,
        hex::encode(esperado),
        "el master_hash sembrado no es el que calcula el servidor"
    );
}

/// El navegador (WASM, `gui/src/lib/crypto_utils.js`) y el escritorio
/// (comando Tauri `derive_keys`) decodifican el hex de la sal a los 16 bytes
/// crudos antes de derivar. Este test replica exactamente ese cálculo: si el
/// seed derivara sobre los bytes ASCII del hex, el `master_hash` no
/// coincidiría y `demo / DemoPass123!` daría 401 en la UI (además del
/// contenido indescifrable).
#[tokio::test]
async fn seed_demo_desbloquea_como_el_navegador() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("demo");
    seed(&data_dir);

    let db = pool(&data_dir).await;
    let (master_hash, salt): (String, String) =
        sqlx::query_as("SELECT master_hash, salt FROM users WHERE username = 'demo'")
            .fetch_one(&db)
            .await
            .unwrap();

    // Como el navegador: hex -> 16 bytes crudos.
    let salt_raw = salt_cruda();
    assert_eq!(salt_raw.len(), 16);
    let (master_key, auth_key) = derivar_claves_dual(DEMO_PASSWORD, &salt_raw).unwrap();

    // Como el servidor al autenticar: deriva sobre el hex de la authKey con
    // los bytes ASCII de la sal.
    let esperado = derivar_masterkey(&hex::encode(&auth_key), salt.as_bytes()).unwrap();
    assert_eq!(
        master_hash,
        hex::encode(esperado),
        "el navegador no podría desbloquear la demo: el seed deriva sobre otra sal"
    );

    // Y con esa misma master_key se descifra el contenido sembrado.
    let titulo: String = sqlx::query_scalar("SELECT title FROM accounts LIMIT 1")
        .fetch_one(&db)
        .await
        .unwrap();
    let claro = abrir_dato_json(&titulo, &master_key)
        .expect("el navegador no podría descifrar las cuentas de la demo");
    let valor: String = serde_json::from_slice(&claro).unwrap();
    assert!(!valor.is_empty());
}

#[tokio::test]
async fn seed_demo_deja_la_boveda_valida() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("demo");
    seed(&data_dir);

    let vault = data_dir.join("pact.pcty");
    assert!(vault.exists(), "falta {}", vault.display());

    // Sin esta firma `validate_vault` falla y la UI arranca pidiendo crear o
    // abrir bóveda en vez de listar el usuario de la demo.
    let storage = StorageManager::new(None);
    assert!(storage.validate_vault(&vault).unwrap());

    // La base del usuario va dentro del vault. Al abrir una bóveda, la app hace
    // `db.reset()` y extrae el primer `<usuario>.db` que encuentre; si faltara,
    // la demo se quedaría vacía justo al abrirla.
    let zip = zip::ZipArchive::new(File::open(&vault).unwrap()).unwrap();
    let mut nombres: Vec<String> = zip.file_names().map(|n| n.to_string()).collect();
    nombres.sort();
    assert_eq!(nombres, vec!["demo.db", "pact.sig"]);
}