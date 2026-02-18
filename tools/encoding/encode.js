#!/usr/bin/env node
/**
 * Kino - HLS Encoding Tool
 *
 * Node.js wrapper for FFmpeg HLS encoding with progress tracking
 *
 * Usage:
 *   node encode.js input.mp4 output_dir [options]
 *
 * Options:
 *   --preset   Encoding preset (ultrafast, fast, medium, slow)
 *   --segment  Segment duration in seconds
 *   --quality  Quality preset (low, medium, high, adaptive)
 */

const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// Kino Branding colors
const colors = {
    purple: '\x1b[35m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    red: '\x1b[31m',
    reset: '\x1b[0m',
    bold: '\x1b[1m',
};

// Quality presets
const QUALITY_PRESETS = {
    low: [
        { height: 360, videoBitrate: '800k', audioBitrate: '96k' },
        { height: 480, videoBitrate: '1400k', audioBitrate: '128k' },
    ],
    medium: [
        { height: 360, videoBitrate: '800k', audioBitrate: '96k' },
        { height: 480, videoBitrate: '1400k', audioBitrate: '128k' },
        { height: 720, videoBitrate: '2800k', audioBitrate: '128k' },
    ],
    high: [
        { height: 360, videoBitrate: '800k', audioBitrate: '96k' },
        { height: 480, videoBitrate: '1400k', audioBitrate: '128k' },
        { height: 720, videoBitrate: '2800k', audioBitrate: '128k' },
        { height: 1080, videoBitrate: '5000k', audioBitrate: '192k' },
    ],
    adaptive: [
        { height: 360, videoBitrate: '800k', audioBitrate: '96k' },
        { height: 480, videoBitrate: '1400k', audioBitrate: '128k' },
        { height: 720, videoBitrate: '2800k', audioBitrate: '128k' },
        { height: 1080, videoBitrate: '5000k', audioBitrate: '192k' },
        { height: 1440, videoBitrate: '8000k', audioBitrate: '256k' },
        { height: 2160, videoBitrate: '15000k', audioBitrate: '256k' },
    ],
};

function printBanner() {
    console.log(colors.purple);
    console.log('╔═══════════════════════════════════════════════════════════╗');
    console.log('║          Purple Squirrel Media - HLS Encoder              ║');
    console.log('╚═══════════════════════════════════════════════════════════╝');
    console.log(colors.reset);
}

function getVideoInfo(inputFile) {
    try {
        const durationCmd = `ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "${inputFile}"`;
        const widthCmd = `ffprobe -v error -select_streams v:0 -show_entries stream=width -of default=noprint_wrappers=1:nokey=1 "${inputFile}"`;
        const heightCmd = `ffprobe -v error -select_streams v:0 -show_entries stream=height -of default=noprint_wrappers=1:nokey=1 "${inputFile}"`;

        return {
            duration: parseFloat(execSync(durationCmd).toString().trim()),
            width: parseInt(execSync(widthCmd).toString().trim()),
            height: parseInt(execSync(heightCmd).toString().trim()),
        };
    } catch (error) {
        throw new Error(`Failed to get video info: ${error.message}`);
    }
}

function formatTime(seconds) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

async function encodeHLS(inputFile, outputDir, options = {}) {
    const {
        preset = 'medium',
        segmentDuration = 4,
        qualityPreset = 'high',
        onProgress = () => {},
    } = options;

    // Get video info
    console.log(`${colors.yellow}Analyzing input file...${colors.reset}`);
    const videoInfo = getVideoInfo(inputFile);
    console.log(`Duration: ${formatTime(videoInfo.duration)}`);
    console.log(`Resolution: ${videoInfo.width}x${videoInfo.height}`);
    console.log();

    // Filter quality levels based on source resolution
    const qualities = QUALITY_PRESETS[qualityPreset].filter(
        (q) => q.height <= videoInfo.height
    );

    console.log(`${colors.green}Encoding ${qualities.length} quality levels:${colors.reset}`);
    qualities.forEach((q) => {
        const width = Math.round((videoInfo.width * q.height) / videoInfo.height / 2) * 2;
        console.log(`  ${q.height}p: ${width}x${q.height} @ ${q.videoBitrate} video, ${q.audioBitrate} audio`);
    });
    console.log();

    // Create output directory
    fs.mkdirSync(outputDir, { recursive: true });

    // Build FFmpeg arguments
    const args = ['-i', inputFile, '-hide_banner', '-y'];

    qualities.forEach((q, i) => {
        const width = Math.round((videoInfo.width * q.height) / videoInfo.height / 2) * 2;
        const maxrate = parseInt(q.videoBitrate) * 1.5 + 'k';
        const bufsize = parseInt(q.videoBitrate) * 2 + 'k';

        args.push(
            '-map', '0:v:0',
            '-map', '0:a:0',
            `-c:v:${i}`, 'libx264',
            '-preset', preset,
            '-crf', '23',
            `-b:v:${i}`, q.videoBitrate,
            `-maxrate:v:${i}`, maxrate,
            `-bufsize:v:${i}`, bufsize,
            `-vf:${i}`, `scale=${width}:${q.height}`,
            `-c:a:${i}`, 'aac',
            `-b:a:${i}`, q.audioBitrate,
            '-ac', '2'
        );
    });

    // Add HLS output for each quality
    qualities.forEach((q, i) => {
        args.push(
            '-f', 'hls',
            '-hls_time', segmentDuration.toString(),
            '-hls_playlist_type', 'vod',
            '-hls_segment_filename', path.join(outputDir, `${q.height}p_%03d.ts`),
            path.join(outputDir, `${q.height}p.m3u8`)
        );
    });

    return new Promise((resolve, reject) => {
        console.log(`${colors.yellow}Starting encode...${colors.reset}`);
        const startTime = Date.now();

        const ffmpeg = spawn('ffmpeg', args);

        let lastProgress = 0;
        ffmpeg.stderr.on('data', (data) => {
            const output = data.toString();

            // Parse progress
            const timeMatch = output.match(/time=(\d+):(\d+):(\d+)/);
            if (timeMatch) {
                const currentTime =
                    parseInt(timeMatch[1]) * 3600 +
                    parseInt(timeMatch[2]) * 60 +
                    parseInt(timeMatch[3]);
                const progress = Math.min(100, (currentTime / videoInfo.duration) * 100);

                if (progress - lastProgress >= 1) {
                    lastProgress = progress;
                    const elapsed = (Date.now() - startTime) / 1000;
                    const eta = progress > 0 ? (elapsed / progress) * (100 - progress) : 0;

                    process.stdout.write(
                        `\r${colors.purple}Progress: ${progress.toFixed(1)}%${colors.reset} | ` +
                        `Elapsed: ${formatTime(elapsed)} | ETA: ${formatTime(eta)}    `
                    );
                    onProgress(progress, currentTime);
                }
            }
        });

        ffmpeg.on('close', (code) => {
            console.log('\n');
            if (code === 0) {
                resolve();
            } else {
                reject(new Error(`FFmpeg exited with code ${code}`));
            }
        });

        ffmpeg.on('error', reject);
    });
}

function createMasterPlaylist(outputDir, qualities, videoInfo) {
    const masterPath = path.join(outputDir, 'master.m3u8');
    let content = '#EXTM3U\n#EXT-X-VERSION:3\n\n';

    qualities.forEach((q) => {
        const width = Math.round((videoInfo.width * q.height) / videoInfo.height / 2) * 2;
        const bandwidth = parseInt(q.videoBitrate) * 1000;

        content += `#EXT-X-STREAM-INF:BANDWIDTH=${bandwidth},RESOLUTION=${width}x${q.height},NAME="${q.height}p"\n`;
        content += `${q.height}p.m3u8\n\n`;
    });

    fs.writeFileSync(masterPath, content);
    return masterPath;
}

async function generateThumbnails(inputFile, outputDir, interval = 10) {
    const thumbDir = path.join(outputDir, 'thumbs');
    fs.mkdirSync(thumbDir, { recursive: true });

    console.log(`${colors.yellow}Generating thumbnails...${colors.reset}`);

    return new Promise((resolve, reject) => {
        const args = [
            '-i', inputFile,
            '-vf', `fps=1/${interval},scale=160:-1`,
            '-hide_banner',
            '-loglevel', 'error',
            path.join(thumbDir, 'thumb_%04d.jpg'),
        ];

        const ffmpeg = spawn('ffmpeg', args);
        ffmpeg.on('close', (code) => {
            if (code === 0) {
                // Create VTT file
                const vttPath = path.join(outputDir, 'thumbnails.vtt');
                let vttContent = 'WEBVTT\n\n';

                const thumbFiles = fs.readdirSync(thumbDir).filter((f) => f.endsWith('.jpg'));
                thumbFiles.forEach((file, i) => {
                    const start = i * interval;
                    const end = (i + 1) * interval;
                    vttContent += `${formatTime(start)}.000 --> ${formatTime(end)}.000\n`;
                    vttContent += `thumbs/${file}\n\n`;
                });

                fs.writeFileSync(vttPath, vttContent);
                resolve(thumbFiles.length);
            } else {
                reject(new Error(`Thumbnail generation failed with code ${code}`));
            }
        });
        ffmpeg.on('error', reject);
    });
}

async function main() {
    printBanner();

    const args = process.argv.slice(2);
    if (args.length < 2) {
        console.log(`${colors.red}Usage: node encode.js <input_file> <output_directory> [options]${colors.reset}`);
        console.log();
        console.log('Options:');
        console.log('  --preset    Encoding preset (ultrafast, fast, medium, slow)');
        console.log('  --segment   Segment duration in seconds (default: 4)');
        console.log('  --quality   Quality preset (low, medium, high, adaptive)');
        console.log();
        console.log('Example:');
        console.log('  node encode.js video.mp4 ./output --preset fast --quality high');
        process.exit(1);
    }

    const inputFile = args[0];
    const outputDir = args[1];

    // Parse options
    const options = {
        preset: 'medium',
        segmentDuration: 4,
        qualityPreset: 'high',
    };

    for (let i = 2; i < args.length; i += 2) {
        switch (args[i]) {
            case '--preset':
                options.preset = args[i + 1];
                break;
            case '--segment':
                options.segmentDuration = parseInt(args[i + 1]);
                break;
            case '--quality':
                options.qualityPreset = args[i + 1];
                break;
        }
    }

    // Validate input file
    if (!fs.existsSync(inputFile)) {
        console.log(`${colors.red}Error: Input file '${inputFile}' not found${colors.reset}`);
        process.exit(1);
    }

    console.log(`${colors.green}Input:${colors.reset} ${inputFile}`);
    console.log(`${colors.green}Output:${colors.reset} ${outputDir}`);
    console.log(`${colors.green}Preset:${colors.reset} ${options.preset}`);
    console.log(`${colors.green}Quality:${colors.reset} ${options.qualityPreset}`);
    console.log();

    try {
        const videoInfo = getVideoInfo(inputFile);
        const qualities = QUALITY_PRESETS[options.qualityPreset].filter(
            (q) => q.height <= videoInfo.height
        );

        // Encode
        await encodeHLS(inputFile, outputDir, options);

        // Create master playlist
        const masterPath = createMasterPlaylist(outputDir, qualities, videoInfo);

        // Generate thumbnails
        const thumbCount = await generateThumbnails(inputFile, outputDir);

        // Summary
        console.log(colors.purple);
        console.log('═══════════════════════════════════════════════════════════');
        console.log(colors.green + 'Encoding complete!' + colors.reset);
        console.log();
        console.log('Output files:');
        console.log(`  Master playlist: ${masterPath}`);
        qualities.forEach((q) => {
            console.log(`  ${q.height}p: ${path.join(outputDir, q.height + 'p.m3u8')}`);
        });
        console.log(`  Thumbnails: ${thumbCount} generated`);
        console.log();
        console.log(`${colors.yellow}To serve locally:${colors.reset} npx serve ${outputDir}`);
        console.log(colors.purple);
        console.log('═══════════════════════════════════════════════════════════');
        console.log(colors.reset);
    } catch (error) {
        console.log(`${colors.red}Error: ${error.message}${colors.reset}`);
        process.exit(1);
    }
}

main();
