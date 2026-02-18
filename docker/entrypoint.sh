#!/bin/sh
set -e

# PSM Player Docker Entrypoint
# Handles environment variable injection and startup

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║          Purple Squirrel Media - PSM Player               ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Inject environment variables into config if needed
if [ -n "$PSM_API_URL" ]; then
    echo "Configuring API URL: $PSM_API_URL"
    # Could be used to inject API endpoint into frontend config
fi

if [ -n "$PSM_ANALYTICS_ID" ]; then
    echo "Analytics ID configured"
fi

# Create runtime config
cat > /usr/share/nginx/html/config.js << EOF
// PSM Player Runtime Configuration
// Generated at container startup
window.PSM_CONFIG = {
    apiUrl: '${PSM_API_URL:-}',
    analyticsId: '${PSM_ANALYTICS_ID:-}',
    version: '${PSM_VERSION:-0.1.0}',
    environment: '${PSM_ENV:-production}'
};
EOF

echo "Starting nginx..."
echo ""

# Execute the main command
exec "$@"
