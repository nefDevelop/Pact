//! Seed de demostración para Pact.
//!
//! Genera una bóveda poblada para poder hacer capturas de pantalla y probar la
//! interfaz sin teclear credenciales reales. Todo el contenido va cifrado con
//! la misma clave que usa el navegador, así que hace falta `pact-core`: si se
//! sembrara en claro, la UI no podría descifrar nada.
//!
//! Uso:
//!
//! ```text
//! cargo run -p pact-server --bin seed_demo -- --data-dir data/demo --force
//! ```
//!
//! Escribe `<data-dir>/pact.db` (el mismo sitio que `DATABASE_URL` por defecto)
//! y `<data-dir>/pact.pcty`, una bóveda válida pero sin la base dentro: la de
//! la demo se genera al abrir la bóveda desde la interfaz.

use std::fs;
use std::path::{Path, PathBuf};

use pact_core::crypto::{derivar_claves_dual, sellar_dato_json};
use pact_server::storage::StorageManager;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

/// Contraseña de la demo. La GUI exige 12 caracteres, mayúscula, número y
/// símbolo (`NewUserModal.svelte`), así que "password" no valdría.
const DEMO_PASSWORD: &str = "DemoPass123!";

struct Args {
    data_dir: PathBuf,
    username: String,
    password: String,
    force: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        data_dir: PathBuf::from("data/demo"),
        username: "demo".to_string(),
        password: DEMO_PASSWORD.to_string(),
        force: false,
    };
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--data-dir" => {
                args.data_dir =
                    PathBuf::from(argv.next().ok_or("--data-dir necesita un valor")?);
            }
            "--username" => {
                args.username = argv.next().ok_or("--username necesita un valor")?;
            }
            "--password" => {
                args.password = argv.next().ok_or("--password necesita un valor")?;
            }
            "--force" => args.force = true,
            "-h" | "--help" => {
                println!(
                    "Uso: seed_demo [--data-dir DIR] [--username U] [--password P] [--force]"
                );
                std::process::exit(0);
            }
            otro => return Err(format!("argumento desconocido: {}", otro)),
        }
    }
    Ok(args)
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("seed_demo: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = parse_args()?;
    let db_path = args.data_dir.join("pact.db");

    if db_path.exists() && !args.force {
        return Err(format!(
            "{} ya existe; pasa --force para regenerarla",
            db_path.display()
        ));
    }
    fs::create_dir_all(&args.data_dir).map_err(|e| format!("creando el directorio: {}", e))?;
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(format!("{}{}", db_path.display(), suffix));
    }

    let started = std::time::Instant::now();

    let pool = open_with_migrations(&db_path).await?;

    // La sal la elige el cliente en `NewUserModal`; aquí es fija para que la
    // demo sea reproducible y se pueda documentar. Se decodifica el hex a los
    // 16 bytes crudos porque los clientes (navegador vía WASM en
    // `crypto_utils.js`, escritorio vía comando Tauri `derive_keys`) derivan
    // sobre los bytes crudos, no sobre el ASCII del hex. Si el seed derivara
    // sobre el ASCII, el usuario de la demo nunca desbloquearía en la UI y el
    // contenido sería indescifrable (ver test
    // `seed_demo_desbloquea_como_el_navegador`).
    let salt = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
    let salt_raw = hex::decode(salt).map_err(|e| format!("sal de la demo inválida: {}", e))?;
    let (master_key, auth_key) = derivar_claves_dual(&args.password, &salt_raw)
        .map_err(|e| format!("derivando las claves: {:?}", e))?;

    // El servidor vuelve a derivar sobre el string hex de la authKey y los bytes
    // ASCII de la sal (ver `auth/handlers.rs`), así que hay que hacerlo igual
    // para que el login de la demo funcione.
    let master_hash = pact_core::crypto::derivar_masterkey(&hex::encode(&auth_key), salt.as_bytes())
        .map_err(|e| format!("derivando el hash de la master key: {:?}", e))?;

    let user_id = insert_user(&pool, &args.username, &hex::encode(master_hash), salt).await?;

    let folders = insert_folders(&pool, user_id, &master_key).await?;
    let cuentas = insert_cuentas(&pool, user_id, &folders, &master_key).await?;
    let totps = insert_totps(&pool, &cuentas, &master_key).await?;
    let borradas = insert_papelera(&pool, user_id, &master_key).await?;

    // La bóveda se crea con la base del usuario dentro, igual que la deja la
    // app al guardar. Esto no es opcional: al abrir una bóveda, `POST
    // /vault/config/path` llama a `db.reset()` y extrae el primer `<usuario>.db`
    // que encuentre. Con una bóveda que solo tenga la firma, la demo se
    // quedaría vacía justo al abrirla.
    let vault_path = args.data_dir.join("pact.pcty");
    let storage = StorageManager::new(Some(vault_path.clone()));
    storage.set_db_path(db_path.clone());
    storage
        .init_vault(&vault_path)
        .map_err(|e| format!("creando la bóveda: {}", e))?;
    storage
        .save_db(&args.username, storage.db_path())
        .map_err(|e| format!("metiendo la base en la bóveda: {}", e))?;

    restrict_permissions(&db_path);
    restrict_permissions(&vault_path);

    pool.close().await;

    resumen(&args, cuentas.len() + borradas, folders.len(), totps, vault_path, started.elapsed());
    Ok(())
}

/// Abre la base y ejecuta las migraciones, igual que el servidor al arrancar:
/// el seed debe dejar el esquema exacto de producción.
async fn open_with_migrations(db_path: &Path) -> Result<sqlx::SqlitePool, String> {
    // La ruta se canonicaliza porque sqlx interpreta los `//` de un URL como
    // separador de host: una ruta relativa con barras (`data/demo/pact.db`)
    // acaba pidiendo abrir el fichero `demo/pact.db` en un host llamado `data`.
    // El servidor nunca sufre esto porque `resolve_db_path` parte del directorio
    // de datos del usuario, que ya es absoluto.
    let dir = db_path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .ok_or_else(|| format!("no se pudo resolver el directorio de {}", db_path.display()))?;
    let ruta = dir.join(db_path.file_name().unwrap_or_default());
    // sqlx no crea el archivo (a diferencia de otros motores), así que se toca
    // antes de conectar. El servidor hace exactamente lo mismo en `main.rs`.
    fs::File::create(&ruta).map_err(|e| format!("creando {}: {}", ruta.display(), e))?;
    let url = format!("sqlite:{}", ruta.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .map_err(|e| format!("abriendo la base: {}", e))?;
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("ejecutando migraciones: {}", e))?;
    Ok(pool)
}

async fn insert_user(
    pool: &sqlx::SqlitePool,
    username: &str,
    master_hash: &str,
    salt: &str,
) -> Result<i64, String> {
    sqlx::query(
        "INSERT INTO users (username, master_hash, salt, color, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(username)
    .bind(master_hash)
    .bind(salt)
    .bind("#6750A4")
    .bind(ahora_desde_hace(400))
    .execute(pool)
    .await
    .map_err(|e| format!("insertando el usuario: {}", e))?;
    sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE username = ?1")
        .bind(username)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("leyendo el id del usuario: {}", e))
}

/// Cifra el texto con la master key.
///
/// La GUI hace `JSON.parse` del resultado, así que el texto plano va serializado
/// como JSON: una cadena es `"Gmail"` con comillas incluidas.
fn cifrar(valor: &str, master_key: &[u8]) -> Result<String, String> {
    let plano = serde_json::to_string(valor).map_err(|e| e.to_string())?;
    // `CryptoError` no implementa Display, así que el error se formatea aquí.
    sellar_dato_json(plano.as_bytes(), master_key).map_err(|e| format!("{:?}", e))
}

struct Carpeta {
    id: i64,
}

async fn insert_folders(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    master_key: &[u8],
) -> Result<Vec<Carpeta>, String> {
    let carpetas = [
        ("Personal", "#6750A4", "user"),
        ("Trabajo", "#00696D", "briefcase"),
        ("Finanzas", "#7D5260", "wallet"),
        ("Desarrollo", "#386A20", "code"),
        ("Redes sociales", "#B3261E", "globe"),
    ];

    let mut out = Vec::new();
    for (i, (nombre, color, icon)) in carpetas.iter().enumerate() {
        let uid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO folders (uid, user_id, name, color, icon, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        )
        .bind(&uid)
        .bind(user_id)
        .bind(cifrar(nombre, master_key)?)
        .bind(color)
        .bind(*icon)
        .bind(ahora_desde_hace(380 - i as i64 * 20))
        .execute(pool)
        .await
        .map_err(|e| format!("insertando la carpeta {}: {}", nombre, e))?;

        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM folders WHERE uid = ?1")
            .bind(&uid)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
        out.push(Carpeta { id });
    }
    Ok(out)
}

struct Cuenta {
    id: i64,
}

async fn insert_cuentas(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    carpetas: &[Carpeta],
    master_key: &[u8],
) -> Result<Vec<Cuenta>, String> {
    // (carpeta, título, url, usuario, contraseña, favorito)
    let datos: [(usize, &str, &str, &str, &str, bool); 18] = [
        (0, "Gmail", "https://mail.google.com", "demo.pact@gmail.com", "k3-R8v!qLm2Zx", true),
        (0, "GitHub", "https://github.com", "pact-demo", "ghp_9xK2mQ7vB1nL4zT6rY8wE3", true),
        (0, "Netflix", "https://netflix.com", "demo@correo.com", "N3tfl!x2024", false),
        (0, "Spotify", "https://spotify.com", "demo.pact", "Sp0tify!demo2024", false),
        (0, "Amazon", "https://amazon.es", "demo@correo.com", "Am4zon$demo", false),
        (0, "Steam", "https://store.steampowered.com", "pact_gamer", "St34m!pass99", false),
        (1, "GitLab", "https://gitlab.com", "pact-dev", "glpat-7Xk2Qp9Rm", true),
        (1, "Jira", "https://pact.atlassian.net", "dev@pact.dev", "J1ra#demo2024", false),
        (1, "AWS Console", "https://console.aws.amazon.com", "pact-ops", "AKIA7XQ2MPLF9KD2", true),
        (1, "Docker Hub", "https://hub.docker.com", "pactdev", "d0cker!hub2024", false),
        (1, "Vercel", "https://vercel.com", "pact-demo", "v3rcel$demo99", false),
        (2, "Banco Demo", "https://banco-demo.es", "ES91 2100 0418 4502 0005 1332", "B4nco!demo2024", true),
        (2, "PayPal", "https://paypal.com", "demo@correo.com", "P4ypal$demo", false),
        (2, " Hacienda", "https://sede.agenciatributaria.gob.es", "12345678Z", "Haci3nda#2024", false),
        (3, "GitHub Copilot", "https://github.com/features/copilot", "pact-dev", "copilot!demo24", false),
        (3, "Docker", "https://docs.docker.com", "pact", "D0cker#pass1", false),
        (4, "Mastodon", "https://mastodon.social", "@pact@pact.dev", "M4stodon$demo", false),
        (4, "LinkedIn", "https://linkedin.com", "pact-demo", "L1nked!n2024", false),
    ];

    let mut out = Vec::new();
    for (i, (carpeta, titulo, url, usuario, pass, favorito)) in datos.iter().enumerate() {
        let uid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO accounts
                (uid, user_id, folder_id, is_favorite, title, url, username_site,
                 encrypted_pass, icon_data, version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, 1, ?9, ?9)",
        )
        .bind(&uid)
        .bind(user_id)
        .bind(carpetas[*carpeta].id)
        .bind(*favorito)
        .bind(cifrar(titulo, master_key)?)
        .bind(cifrar(url, master_key)?)
        .bind(cifrar(usuario, master_key)?)
        .bind(cifrar(pass, master_key)?)
        .bind(ahora_desde_hace(360 - i as i64 * 18))
        .execute(pool)
        .await
        .map_err(|e| format!("insertando la cuenta {}: {}", titulo, e))?;

        let id = sqlx::query_scalar::<_, i64>("SELECT id FROM accounts WHERE uid = ?1")
            .bind(&uid)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
        out.push(Cuenta { id });
    }
    Ok(out)
}

/// Semillas TOTP base32 (16 caracteres = 80 bits, el tamaño habitual de
/// Google Authenticator). Van cifradas igual que el resto.
async fn insert_totps(
    pool: &sqlx::SqlitePool,
    cuentas: &[Cuenta],
    master_key: &[u8],
) -> Result<usize, String> {
    const SEMILLAS: [&str; 3] = [
        "JBSWY3DPEHPK3PXP",
        "KRSXG5CTMVRXEZLU",
        "MFRGGZDFMZTWQ2LK",
    ];

    // TOTP en las tres cuentas que de verdad lo usarían: banca, PayPal y el
    // GitHub del trabajo (el de la cuenta personal es solo de lectura).
    const CON_TOTP: [usize; 3] = [11, 12, 6];

    let mut n = 0;
    for (idx, semilla) in CON_TOTP.iter().zip(SEMILLAS.iter()) {
        let cuenta = &cuentas[*idx];
        sqlx::query(
            "INSERT INTO totp_settings (account_id, encrypted_seed, issuer) VALUES (?1, ?2, ?3)",
        )
        .bind(cuenta.id)
        .bind(cifrar(semilla, master_key)?)
        .bind(cifrar("Pact Demo", master_key)?)
        .execute(pool)
        .await
        .map_err(|e| format!("insertando TOTP: {}", e))?;
        n += 1;
    }
    Ok(n)
}

/// Bóveda con la marca de agua borrada, para que `/` abra la papelera con
/// contenido en vez de una vista vacía.
async fn insert_papelera(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    master_key: &[u8],
) -> Result<usize, String> {
    const BORRADAS: [(&str, &str); 3] = [
        ("Vodafone España", "https://www.vodafone.es"),
        ("Ticketmaster", "https://www.ticketmaster.es"),
        ("Old Blogspot", "https://pact-demo.blogspot.com"),
    ];

    let mut n = 0;
    for (i, (titulo, url)) in BORRADAS.iter().enumerate() {
        let uid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO accounts
                (uid, user_id, folder_id, is_favorite, title, url, username_site,
                 encrypted_pass, icon_data, version, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, NULL, 0, ?3, ?4, ?5, ?6, NULL, 2, ?7, ?8, ?8)",
        )
        .bind(&uid)
        .bind(user_id)
        .bind(cifrar(titulo, master_key)?)
        .bind(cifrar(url, master_key)?)
        .bind(cifrar("usuario-borrado", master_key)?)
        .bind(cifrar("ya-no-entra", master_key)?)
        .bind(ahora_desde_hace(300 - i as i64 * 30))
        .bind(ahora_desde_hace(30 - i as i64 * 10))
        .execute(pool)
        .await
        .map_err(|e| format!("insertando en la papelera: {}", e))?;
        n += 1;
    }
    Ok(n)
}

/// El vault guarda credenciales y la base contiene los metadatos: ambos a 600,
/// como hace el servidor (`main.rs` y `init_vault`).
fn restrict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
            eprintln!(
                "seed_demo: no se pudieron restringir los permisos de {}: {}",
                path.display(),
                e
            );
        }
    }
}

/// Fecha ISO desplazada días hacia atrás, para que los datos tengan antigüedad
/// y no queden todos en el mismo instante.
fn ahora_desde_hace(dias: i64) -> String {
    let momento = chrono_like_now() - (dias as f64) * 86_400.0;
    format_iso(momento)
}

/// `time` no está entre las dependencias del crate, así que se calcula a mano
/// desde el epoch en segundos.
fn chrono_like_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Fecha ISO 8601 en UTC a partir de un epoch en segundos.
fn format_iso(epoch: f64) -> String {
    let secs = epoch as i64;
    let dias = secs.div_euclid(86_400);
    let resto = secs.rem_euclid(86_400);

    // Días desde 1970-01-01 a la fecha civil (algoritmo de Howard Hinnant).
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

fn resumen(
    args: &Args,
    cuentas: usize,
    carpetas: usize,
    totps: usize,
    vault_path: PathBuf,
    took: std::time::Duration,
) {
    println!();
    println!("Bóveda de demostración lista");
    println!("  {} carpetas · {} cuentas · {} con TOTP", carpetas, cuentas, totps);
    println!("  usuario: {} / {}", args.username, args.password);
    println!("  base:    {}", args.data_dir.join("pact.db").display());
    println!("  vault:   {}", vault_path.display());
    println!("  generado en {:?}", took);
}