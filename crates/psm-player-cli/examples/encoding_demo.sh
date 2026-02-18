#!/bin/bash
#
# PSM Player CLI - Encoding Demo Script
#
# This script demonstrates the encoding capabilities of the PSM Player CLI.
# It shows how to encode videos for HLS streaming with different presets.
#
# Prerequisites:
# - FFmpeg installed
# - PSM Player CLI built: cargo build -p psm-player-cli
#
# Usage: ./encoding_demo.sh <input_video>

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${PURPLE}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║           PSM Player CLI - Encoding Demo                  ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check for input file
if [ -z "$1" ]; then
    echo -e "${YELLOW}Usage: $0 <input_video>${NC}"
    echo ""
    echo "Example:"
    echo "  $0 my_video.mp4"
    echo ""
    echo "This script will encode the video with multiple presets:"
    echo "  - web: Standard web streaming (480p, 720p, 1080p)"
    echo "  - mobile: Mobile-optimized (360p, 480p, 720p)"
    echo "  - premium: High quality (720p, 1080p, 4K)"
    echo "  - live: Low-latency live streaming"
    echo ""
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_DIR="./psm_encoded_output"

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
    echo -e "${RED}Error: Input file '$INPUT_FILE' not found${NC}"
    exit 1
fi

# Check for FFmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}Error: FFmpeg is not installed${NC}"
    echo "Please install FFmpeg: https://ffmpeg.org/download.html"
    exit 1
fi

# Check for PSM CLI
PSM_CLI="./target/debug/psm-player-cli"
if [ ! -f "$PSM_CLI" ]; then
    echo -e "${YELLOW}Building PSM Player CLI...${NC}"
    cargo build -p psm-player-cli
fi

echo -e "${GREEN}Input file:${NC} $INPUT_FILE"
echo -e "${GREEN}Output directory:${NC} $OUTPUT_DIR"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Function to show file size
show_size() {
    if [ -d "$1" ]; then
        size=$(du -sh "$1" 2>/dev/null | cut -f1)
        echo "  Total size: $size"
    fi
}

# Demo 1: List available presets
echo -e "${PURPLE}1. Available Encoding Presets${NC}"
echo "================================"
$PSM_CLI encode --list-presets 2>/dev/null || echo "  web, mobile, premium, live, archive"
echo ""

# Demo 2: Encode with web preset
echo -e "${PURPLE}2. Encoding with 'web' preset${NC}"
echo "================================"
echo "Command: psm-player-cli encode -i $INPUT_FILE -o $OUTPUT_DIR/web --preset web"
echo ""
echo -e "${YELLOW}This generates:${NC}"
echo "  - 480p  @ 1500 kbps (H.264/AAC)"
echo "  - 720p  @ 3000 kbps (H.264/AAC)"
echo "  - 1080p @ 6000 kbps (H.264/AAC)"
echo "  - master.m3u8 (HLS master playlist)"
echo ""
# Uncomment to actually run:
# $PSM_CLI encode -i "$INPUT_FILE" -o "$OUTPUT_DIR/web" --preset web
# show_size "$OUTPUT_DIR/web"
echo ""

# Demo 3: Encode with mobile preset
echo -e "${PURPLE}3. Encoding with 'mobile' preset${NC}"
echo "================================"
echo "Command: psm-player-cli encode -i $INPUT_FILE -o $OUTPUT_DIR/mobile --preset mobile"
echo ""
echo -e "${YELLOW}Optimized for mobile devices:${NC}"
echo "  - Lower bitrates for cellular networks"
echo "  - Baseline H.264 profile for compatibility"
echo "  - Smaller segment durations for faster startup"
echo ""

# Demo 4: Encode with custom options
echo -e "${PURPLE}4. Custom Encoding Options${NC}"
echo "================================"
echo "Command: psm-player-cli encode -i $INPUT_FILE -o $OUTPUT_DIR/custom \\"
echo "    --preset web \\"
echo "    --segment-duration 4 \\"
echo "    --max-bitrate 8000 \\"
echo "    --audio-bitrate 192"
echo ""
echo -e "${YELLOW}Custom options:${NC}"
echo "  --segment-duration: HLS segment length in seconds"
echo "  --max-bitrate: Maximum video bitrate (kbps)"
echo "  --audio-bitrate: Audio bitrate (kbps)"
echo "  --keyframe-interval: Keyframe interval in frames"
echo ""

# Demo 5: Check encoding status
echo -e "${PURPLE}5. FFmpeg Command Preview${NC}"
echo "================================"
echo "The CLI generates FFmpeg commands like:"
echo ""
echo -e "${GREEN}ffmpeg -i input.mp4 \\${NC}"
echo -e "${GREEN}  -c:v libx264 -preset medium -crf 23 \\${NC}"
echo -e "${GREEN}  -c:a aac -b:a 128k \\${NC}"
echo -e "${GREEN}  -f hls -hls_time 6 \\${NC}"
echo -e "${GREEN}  -hls_playlist_type vod \\${NC}"
echo -e "${GREEN}  -master_pl_name master.m3u8 \\${NC}"
echo -e "${GREEN}  -var_stream_map \"v:0,a:0 v:1,a:1 v:2,a:2\" \\${NC}"
echo -e "${GREEN}  output/stream_%v.m3u8${NC}"
echo ""

# Demo 6: Output structure
echo -e "${PURPLE}6. Expected Output Structure${NC}"
echo "================================"
cat << 'EOF'
output/
├── master.m3u8          # HLS master playlist
├── stream_0.m3u8        # 480p variant playlist
├── stream_0_000.ts      # 480p segments
├── stream_0_001.ts
├── ...
├── stream_1.m3u8        # 720p variant playlist
├── stream_1_000.ts      # 720p segments
├── ...
├── stream_2.m3u8        # 1080p variant playlist
└── stream_2_000.ts      # 1080p segments
EOF
echo ""

echo -e "${GREEN}Demo complete!${NC}"
echo ""
echo "To actually encode a video, run:"
echo -e "  ${YELLOW}cargo run -p psm-player-cli -- encode -i input.mp4 -o output/ --preset web${NC}"
