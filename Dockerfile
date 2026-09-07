# PSM Player - Multi-stage Docker Build
#
# Builds the web player and serves it via nginx
#
# Usage:
#   docker build -t kino .
#   docker run -p 8080:80 kino

# ============================================================================
# Stage 1: Build WASM module
# ============================================================================
# rust:1.75 predates Cargo.lock format v4 (Rust 1.78), which this workspace
# now uses, so the pinned image failed before compiling anything:
#   error: failed to parse lock file at: /app/Cargo.lock
# Tracking the 1.x line keeps this stage from bit-rotting again as the
# lockfile format moves. There is no -D warnings gate here, so a moving
# toolchain costs nothing.
FROM rust:1-slim AS wasm-builder

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
# The root Cargo.toml declares eight workspace members, and `cargo metadata`
# loads every one of them even though wasm-pack only builds kino-wasm. Copying
# just the two used crates leaves the rest unresolvable:
#   error: failed to load manifest for workspace member `/app/crates/kino-desktop`
# Only kino-wasm is compiled, so the heavier crates' native dependencies
# (gstreamer, gtk) are never needed here.
COPY crates ./crates

# Build WASM package
WORKDIR /app/crates/kino-wasm
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
COPY --from=wasm-builder /app/crates/kino-wasm/pkg /usr/share/nginx/html/wasm/

# Create custom entrypoint for environment variable injection
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:80/health || exit 1

EXPOSE 80

ENTRYPOINT ["/entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
