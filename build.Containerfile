# Docker/Podman Containerfile para compilar Pact (Tauri v2)
# Usamos Ubuntu 22.04 LTS por ser el estándar de compatibilidad para AppImage
FROM mirror.gcr.io/library/ubuntu:22.04

# Evitar diálogos interactivos durante la instalación
ENV DEBIAN_FRONTEND=noninteractive

# 1. Instalar dependencias esenciales de Tauri v2 y empaquetado AppImage
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    build-essential \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    squashfs-tools \
    cmake \
    git \
    file \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# 2. Instalar Rust (Toolchain estable)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 3. Instalar Node.js 20 (LTS)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# 4. Preparar entorno de trabajo
WORKDIR /app

# --- CONFIGURACIÓN PARA CONTENEDORES ---
# Permite que linuxdeploy se extraiga y ejecute sin FUSE (fundamental en Podman)
ENV APPIMAGE_EXTRACT_AND_RUN=1
# Evita fallos con librerías modernas de host al empaquetar
ENV NO_STRIP=1

# Comando por defecto: compilar servidor y desktop
CMD ["make", "build"]
