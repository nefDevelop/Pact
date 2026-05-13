-- Tabla de Usuarios
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username VARCHAR(100) NOT NULL UNIQUE,
    master_hash VARCHAR(255) NOT NULL,
    salt VARCHAR(255) NOT NULL,
    color VARCHAR(20) DEFAULT '#6750A4',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tabla de Carpetas
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uid VARCHAR(36) NOT NULL UNIQUE,
    user_id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    color VARCHAR(20) DEFAULT '#000000',
    icon VARCHAR(50) DEFAULT 'folder',
    version INTEGER DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Tabla de Cuentas (Credenciales)
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uid VARCHAR(36) NOT NULL UNIQUE,
    user_id INTEGER NOT NULL,
    folder_id INTEGER,
    is_favorite BOOLEAN DEFAULT 0,
    title VARCHAR(255) NOT NULL,
    url VARCHAR(500),
    username_site VARCHAR(255),
    encrypted_pass VARCHAR(500) NOT NULL,
    icon_data TEXT,
    version INTEGER DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
);

-- Tabla de Configuración TOTP
CREATE TABLE IF NOT EXISTS totp_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL UNIQUE,
    encrypted_seed VARCHAR(500) NOT NULL,
    issuer VARCHAR(100),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Tabla de Cuentas Pendientes (Limbo)
CREATE TABLE IF NOT EXISTS pending_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username VARCHAR(100) NOT NULL,
    data_blob TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Índices para optimizar búsquedas
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_folders_user_id ON folders(user_id);
CREATE INDEX IF NOT EXISTS idx_accounts_user_id ON accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_accounts_folder_id ON accounts(folder_id);
CREATE INDEX IF NOT EXISTS idx_pending_username ON pending_accounts(username);
