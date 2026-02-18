# PSM Player - Multi-stage Docker Build
#
# Builds the web player and serves it via nginx
#
# Usage:
#   docker build -t psm-player .
#   docker run -p 8080:80 psm-player

# ============================================================================
# Stage 1: Build WASM module
# ============================================================================
FROM rust:1.75-slim AS wasm-builder

# Install wasm-pack and build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/psm-player-core ./crates/psm-player-core
COPY crates/psm-player-wasm ./crates/psm-player-wasm

# Build WASM package
WORKDIR /app/crates/psm-player-wasm
RUN wasm-pack build --target web --release

# ============================================================================
# Stage 2: Production image with nginx
# ============================================================================
FROM nginx:alpine AS production

# Install envsubst for environment variable substitution
RUN apk add --no-cache gettext

# Copy nginx configuration
COPY docker/nginx.conf /etc/nginx/nginx.conf

# Copy web player files
COPY web /usr/share/nginx/html/

# Copy built WASM files
COPY --from=wasm-builder /app/crates/psm-player-wasm/pkg /usr/share/nginx/html/wasm/

# Create custom entrypoint for environment variable injection
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:80/health || exit 1

EXPOSE 80

ENTRYPOINT ["/entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
