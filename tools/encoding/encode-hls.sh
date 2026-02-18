#!/bin/bash
#
# PSM Player - HLS Encoding Script
# Encodes video files to multi-bitrate HLS with adaptive streaming
#
# Usage: ./encode-hls.sh input.mp4 output_dir
#
# Requirements: ffmpeg with libx264 and libfdk_aac (or aac)
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# PSM Branding
echo -e "${PURPLE}"
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║          Purple Squirrel Media - HLS Encoder              ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Usage: $0 <input_file> <output_directory>${NC}"
    echo ""
    echo "Options:"
    echo "  -p, --preset    Encoding preset (ultrafast, fast, medium, slow)"
    echo "  -s, --segment   Segment duration in seconds (default: 4)"
    echo "  -c, --chapters  Generate chapter markers (requires input with chapters)"
    echo ""
    echo "Example:"
    echo "  $0 video.mp4 ./output"
    echo "  $0 video.mp4 ./output --preset fast --segment 6"
    exit 1
fi

INPUT="$1"
OUTPUT_DIR="$2"
shift 2

# Default options
PRESET="medium"
SEGMENT_DURATION=4
INCLUDE_CHAPTERS=false

# Parse optional arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--preset)
            PRESET="$2"
            shift 2
            ;;
        -s|--segment)
            SEGMENT_DURATION="$2"
            shift 2
            ;;
        -c|--chapters)
            INCLUDE_CHAPTERS=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Check if input file exists
if [ ! -f "$INPUT" ]; then
    echo -e "${RED}Error: Input file '$INPUT' not found${NC}"
    exit 1
fi

# Check for ffmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}Error: ffmpeg not found. Please install ffmpeg.${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${GREEN}Input:${NC} $INPUT"
echo -e "${GREEN}Output:${NC} $OUTPUT_DIR"
echo -e "${GREEN}Preset:${NC} $PRESET"
echo -e "${GREEN}Segment Duration:${NC} ${SEGMENT_DURATION}s"
echo ""

# Get video info
echo -e "${YELLOW}Analyzing input file...${NC}"
DURATION=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$INPUT")
WIDTH=$(ffprobe -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 "$INPUT")
HEIGHT=$(ffprobe -v error -select_streams v:0 -show_entries stream=height -of default=noprint_wrappers=1:nokey=1 "$INPUT")

echo -e "Duration: ${DURATION}s"
echo -e "Resolution: ${WIDTH}x${HEIGHT}"
echo ""

# Quality levels (height, video bitrate, audio bitrate)
declare -a QUALITIES=(
    "360:800k:96k"
    "480:1400k:128k"
    "720:2800k:128k"
    "1080:5000k:192k"
)

# Add 4K if source is 4K
if [ "$HEIGHT" -ge 2160 ]; then
    QUALITIES+=("2160:15000k:256k")
fi

# Filter qualities based on source resolution
FILTERED_QUALITIES=()
for q in "${QUALITIES[@]}"; do
    IFS=':' read -r h vb ab <<< "$q"
    if [ "$HEIGHT" -ge "$h" ]; then
        FILTERED_QUALITIES+=("$q")
    fi
done

echo -e "${YELLOW}Encoding ${#FILTERED_QUALITIES[@]} quality levels...${NC}"

# Build ffmpeg command
FFMPEG_CMD="ffmpeg -i \"$INPUT\" -hide_banner"

# Add video filters and outputs for each quality
OUTPUT_NAMES=()
for i in "${!FILTERED_QUALITIES[@]}"; do
    IFS=':' read -r h vb ab <<< "${FILTERED_QUALITIES[$i]}"

    # Calculate width (maintain aspect ratio, ensure even)
    w=$(echo "scale=0; ($WIDTH * $h / $HEIGHT + 1) / 2 * 2" | bc)

    OUTPUT_NAME="${h}p"
    OUTPUT_NAMES+=("$OUTPUT_NAME")

    echo -e "  ${GREEN}${OUTPUT_NAME}:${NC} ${w}x${h} @ ${vb} video, ${ab} audio"

    FFMPEG_CMD+=" \
        -map 0:v:0 -map 0:a:0 \
        -c:v:$i libx264 -preset $PRESET -crf 23 \
        -b:v:$i $vb -maxrate:v:$i $(echo "$vb" | sed 's/k/*1.5/' | bc)k -bufsize:v:$i $(echo "$vb" | sed 's/k/*2/' | bc)k \
        -vf:$i \"scale=$w:$h\" \
        -c:a:$i aac -b:a:$i $ab -ac 2 \
        -f hls \
        -hls_time $SEGMENT_DURATION \
        -hls_playlist_type vod \
        -hls_segment_filename \"$OUTPUT_DIR/${OUTPUT_NAME}_%03d.ts\" \
        \"$OUTPUT_DIR/${OUTPUT_NAME}.m3u8\""
done

echo ""
echo -e "${YELLOW}Starting encode...${NC}"
echo ""

# Run ffmpeg
eval $FFMPEG_CMD

# Create master playlist
echo -e "${YELLOW}Creating master playlist...${NC}"

MASTER_PLAYLIST="$OUTPUT_DIR/master.m3u8"
echo "#EXTM3U" > "$MASTER_PLAYLIST"
echo "#EXT-X-VERSION:3" >> "$MASTER_PLAYLIST"
echo "" >> "$MASTER_PLAYLIST"

for i in "${!FILTERED_QUALITIES[@]}"; do
    IFS=':' read -r h vb ab <<< "${FILTERED_QUALITIES[$i]}"
    w=$(echo "scale=0; ($WIDTH * $h / $HEIGHT + 1) / 2 * 2" | bc)
    bw=$(echo "$vb" | sed 's/k/000/')

    echo "#EXT-X-STREAM-INF:BANDWIDTH=$bw,RESOLUTION=${w}x${h},NAME=\"${h}p\"" >> "$MASTER_PLAYLIST"
    echo "${OUTPUT_NAMES[$i]}.m3u8" >> "$MASTER_PLAYLIST"
    echo "" >> "$MASTER_PLAYLIST"
done

# Extract chapters if requested
if [ "$INCLUDE_CHAPTERS" = true ]; then
    echo -e "${YELLOW}Extracting chapters...${NC}"
    ffprobe -v error -show_chapters -of json "$INPUT" > "$OUTPUT_DIR/chapters.json" 2>/dev/null || true
fi

# Generate thumbnail sprites
echo -e "${YELLOW}Generating thumbnails...${NC}"
THUMB_DIR="$OUTPUT_DIR/thumbs"
mkdir -p "$THUMB_DIR"

# Generate thumbnails every 10 seconds
ffmpeg -i "$INPUT" -vf "fps=1/10,scale=160:-1" -hide_banner -loglevel error "$THUMB_DIR/thumb_%04d.jpg"

# Create thumbnail VTT file
THUMB_VTT="$OUTPUT_DIR/thumbnails.vtt"
echo "WEBVTT" > "$THUMB_VTT"
echo "" >> "$THUMB_VTT"

THUMB_COUNT=$(ls -1 "$THUMB_DIR"/thumb_*.jpg 2>/dev/null | wc -l)
for i in $(seq 1 $THUMB_COUNT); do
    START=$((($i - 1) * 10))
    END=$(($i * 10))
    printf "%02d:%02d:%02d.000 --> %02d:%02d:%02d.000\n" \
        $((START/3600)) $(((START%3600)/60)) $((START%60)) \
        $((END/3600)) $(((END%3600)/60)) $((END%60)) >> "$THUMB_VTT"
    printf "thumbs/thumb_%04d.jpg\n\n" $i >> "$THUMB_VTT"
done

# Summary
echo ""
echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Encoding complete!${NC}"
echo ""
echo "Output files:"
echo "  Master playlist: $OUTPUT_DIR/master.m3u8"
for name in "${OUTPUT_NAMES[@]}"; do
    echo "  ${name}: $OUTPUT_DIR/${name}.m3u8"
done
echo "  Thumbnails: $OUTPUT_DIR/thumbnails.vtt"
if [ "$INCLUDE_CHAPTERS" = true ]; then
    echo "  Chapters: $OUTPUT_DIR/chapters.json"
fi
echo ""
echo -e "To serve locally: ${YELLOW}python3 -m http.server 8080 -d $OUTPUT_DIR${NC}"
echo -e "${PURPLE}═══════════════════════════════════════════════════════════${NC}"
