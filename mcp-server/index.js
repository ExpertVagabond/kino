#!/usr/bin/env node
/**
 * kino-mcp — MCP server for video stream analysis, QC, and monitoring.
 *
 * @security
 * - CLI path validation: KINO_CLI_PATH must be absolute, no shell metacharacters
 * - No shell execution: uses execFile (not exec) to prevent injection
 * - Input URLs validated via Zod `.url()` schema before passing to CLI
 * - Process timeout: 30 s default, 10 MB max buffer to prevent resource exhaustion
 * - Environment variables passed through — no secrets constructed in code
 * - Error output from child process is bounded and never includes raw env
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { existsSync } from "node:fs";

const execFileAsync = promisify(execFile);

// ---------------------------------------------------------------------------
// Security: constants
// ---------------------------------------------------------------------------

/** Maximum allowed CLI argument length — prevents oversized inputs. */
const MAX_ARG_LENGTH = 4096;

/** Maximum child process output buffer (10 MB). */
const MAX_BUFFER = 10 * 1024 * 1024;

/** Default child process timeout (30 seconds). */
const DEFAULT_TIMEOUT_MS = 30_000;

// ---------------------------------------------------------------------------
// Resolve kino-cli binary path (security: validated)
// ---------------------------------------------------------------------------

function resolveKinoCli() {
  // 1. Explicit env override
  if (process.env.KINO_CLI_PATH) {
    const cliPath = process.env.KINO_CLI_PATH;
    // Validate: must be an absolute path, no shell metacharacters
    if (!cliPath.startsWith('/') || /[;|`$\n\r]/.test(cliPath)) {
      throw new Error('KINO_CLI_PATH must be an absolute path without shell metacharacters');
    }
    if (!existsSync(cliPath)) {
      throw new Error(`KINO_CLI_PATH not found: ${cliPath}`);
    }
    return cliPath;
  }

  // 2. Common install locations
  const candidates = [
    "/usr/local/bin/kino-cli",
    "/opt/homebrew/bin/kino-cli",
    `${process.env.HOME}/.cargo/bin/kino-cli`,
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }

  // 3. Fall back to bare name (relies on PATH)
  return "kino-cli";
}

const KINO_CLI = resolveKinoCli();

// ---------------------------------------------------------------------------
// Helper — run kino-cli and return parsed output
// ---------------------------------------------------------------------------

async function runKinoCli(args, { timeoutMs = DEFAULT_TIMEOUT_MS } = {}) {
  // Validate all CLI arguments against size limit to prevent oversized inputs
  for (const arg of args) {
    if (typeof arg !== "string" || arg.length > MAX_ARG_LENGTH) {
      return { ok: false, data: null, raw: `Argument exceeds max length (${MAX_ARG_LENGTH})` };
    }
  }
  try {
    const { stdout, stderr } = await execFileAsync(KINO_CLI, args, {
      timeout: timeoutMs,
      maxBuffer: MAX_BUFFER,
      env: { ...process.env },
    });

    // Try JSON parse first, fall back to raw text
    const text = stdout.trim();
    try {
      return { ok: true, data: JSON.parse(text), raw: text };
    } catch {
      return { ok: true, data: null, raw: text };
    }
  } catch (err) {
    const message = err.stderr?.trim() || err.message || "Unknown error";
    return { ok: false, data: null, raw: message };
  }
}

function formatResult(result) {
  if (!result.ok) {
    return { content: [{ type: "text", text: `Error: ${result.raw}` }], isError: true };
  }
  if (result.data) {
    return { content: [{ type: "text", text: JSON.stringify(result.data, null, 2) }] };
  }
  return { content: [{ type: "text", text: result.raw }] };
}

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

const server = new McpServer({
  name: "kino-mcp",
  version: "0.1.0",
});

// ── 1. analyze_stream ─────────────────────────────────────────────────────

server.tool(
  "analyze_stream",
  "Parse an HLS/DASH manifest and return renditions, codecs, duration, and live status",
  { url: z.string().url().describe("HLS or DASH manifest URL") },
  async ({ url }) => {
    const result = await runKinoCli(["analyze", url]);
    return formatResult(result);
  }
);

// ── 2. validate_stream ────────────────────────────────────────────────────

server.tool(
  "validate_stream",
  "Check segment accessibility and bitrate conformance for a stream",
  {
    url: z.string().url().describe("HLS or DASH manifest URL"),
    segments: z
      .number()
      .int()
      .positive()
      .optional()
      .describe("Number of segments to validate (default: all)"),
  },
  async ({ url, segments }) => {
    const args = ["validate", url];
    if (segments !== undefined) args.push("--segments", String(segments));
    const result = await runKinoCli(args);
    return formatResult(result);
  }
);

// ── 3. quality_check ──────────────────────────────────────────────────────

server.tool(
  "quality_check",
  "Run a full QC report — DRM status, captions, bitrate ladder conformance",
  {
    url: z.string().url().describe("HLS or DASH manifest URL"),
    output_format: z
      .enum(["json", "text"])
      .optional()
      .default("json")
      .describe("Output format for the report"),
  },
  async ({ url, output_format }) => {
    const args = ["qc", url, "--output-format", output_format ?? "json"];
    const result = await runKinoCli(args);
    return formatResult(result);
  }
);

// ── 4. monitor_stream ─────────────────────────────────────────────────────

server.tool(
  "monitor_stream",
  "Check live stream health — latency, segment freshness, error rate",
  {
    url: z.string().url().describe("Live stream manifest URL"),
    interval: z
      .number()
      .int()
      .positive()
      .optional()
      .default(5)
      .describe("Polling interval in seconds"),
  },
  async ({ url, interval }) => {
    const args = [
      "monitor",
      url,
      "--interval",
      String(interval ?? 5),
      "--count",
      "1",
    ];
    const result = await runKinoCli(args, { timeoutMs: 60_000 });
    return formatResult(result);
  }
);

// ── 5. fingerprint_audio ──────────────────────────────────────────────────

server.tool(
  "fingerprint_audio",
  "Generate an audio fingerprint for content identification and duplicate detection",
  {
    file_path: z.string().min(1).max(1024).describe("Path to audio or video file"),
  },
  async ({ file_path }) => {
    if (/[;|`$\n\r]/.test(file_path) || file_path.includes('\0')) {
      return { content: [{ type: "text", text: "Error: Invalid characters in file path" }], isError: true };
    }
    const result = await runKinoCli(["fingerprint", file_path]);
    return formatResult(result);
  }
);

// ── 6. autotag_content ────────────────────────────────────────────────────

server.tool(
  "autotag_content",
  "Auto-detect genre, mood, and BPM from audio content",
  {
    file_path: z.string().min(1).max(1024).describe("Path to audio or video file"),
  },
  async ({ file_path }) => {
    if (/[;|`$\n\r]/.test(file_path) || file_path.includes('\0')) {
      return { content: [{ type: "text", text: "Error: Invalid characters in file path" }], isError: true };
    }
    const result = await runKinoCli(["autotag", file_path]);
    return formatResult(result);
  }
);

// ── 7. compare_streams ────────────────────────────────────────────────────

server.tool(
  "compare_streams",
  "Compare two streams for quality differences — resolution, bitrate, codec mismatches",
  {
    url_a: z.string().url().describe("First stream manifest URL"),
    url_b: z.string().url().describe("Second stream manifest URL"),
  },
  async ({ url_a, url_b }) => {
    const result = await runKinoCli(["compare", url_a, url_b]);
    return formatResult(result);
  }
);

// ── 8. encode_video ───────────────────────────────────────────────────────

server.tool(
  "encode_video",
  "Generate encoding presets for a target platform — returns ffmpeg flags and HLS/DASH config",
  {
    preset: z
      .enum(["mobile", "desktop", "4k", "low-bandwidth"])
      .describe("Target encoding preset"),
  },
  async ({ preset }) => {
    const result = await runKinoCli(["preset", preset]);
    return formatResult(result);
  }
);

// ---------------------------------------------------------------------------
// Start
// ---------------------------------------------------------------------------

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("kino-mcp fatal:", err);
  process.exit(1);
});
