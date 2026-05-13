-- Tabla de Configuración de Sincronización
CREATE TABLE IF NOT EXISTS sync_configs (
    user_id INTEGER PRIMARY KEY,
    remote_host VARCHAR(255) NOT NULL,
    remote_port INTEGER NOT NULL DEFAULT 3306,
    remote_user VARCHAR(100) NOT NULL,
    remote_pass VARCHAR(255) NOT NULL,
    remote_name VARCHAR(100) NOT NULL,
    preferred_mode VARCHAR(20) DEFAULT 'local',
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
