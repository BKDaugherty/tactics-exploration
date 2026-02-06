#!/usr/bin/env node

import { resolve } from "path";
import { homedir } from "os";

const cdpPath = resolve(
  homedir(),
  ".pi/agent/git/github.com/mitsuhiko/agent-stuff/skills/web-browser/scripts/cdp.js"
);
const { connect } = await import(cdpPath);

function usage() {
  console.error("Usage: wait-for-log.js <pattern> [--timeout <ms>]");
  console.error('Example: wait-for-log.js "[GAME_STATE] entered Battle" --timeout 20000');
}

function parseArgs(argv) {
  const positional = [];
  let timeoutMs = 15000;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--timeout") {
      const value = argv[i + 1];
      if (!value) {
        throw new Error("Missing value for --timeout");
      }
      const parsed = Number.parseInt(value, 10);
      if (Number.isNaN(parsed) || parsed <= 0) {
        throw new Error(`Invalid timeout: ${value}`);
      }
      timeoutMs = parsed;
      i += 1;
      continue;
    }

    positional.push(arg);
  }

  const pattern = positional.join(" ").trim();
  if (!pattern) {
    throw new Error("Missing pattern");
  }

  return { pattern, timeoutMs };
}

function remoteArgToText(arg) {
  if (!arg || typeof arg !== "object") return String(arg);
  if (Object.prototype.hasOwnProperty.call(arg, "value")) return String(arg.value);
  if (Object.prototype.hasOwnProperty.call(arg, "unserializableValue")) {
    return String(arg.unserializableValue);
  }
  if (Object.prototype.hasOwnProperty.call(arg, "description")) return String(arg.description);
  if (arg.type === "undefined") return "undefined";
  return JSON.stringify(arg);
}

let cdp;
let cleanupHandlers = [];
let timeoutId;
let finished = false;

function finish(code, message, error = false) {
  if (finished) return;
  finished = true;

  if (timeoutId) clearTimeout(timeoutId);
  for (const cleanup of cleanupHandlers) {
    try {
      cleanup();
    } catch {
      // Ignore cleanup errors.
    }
  }
  cleanupHandlers = [];

  if (cdp) {
    try {
      cdp.close();
    } catch {
      // Ignore close errors.
    }
  }

  if (message) {
    if (error) {
      console.error(message);
    } else {
      console.log(message);
    }
  }

  process.exit(code);
}

try {
  const { pattern, timeoutMs } = parseArgs(process.argv.slice(2));

  timeoutId = setTimeout(() => {
    finish(1, `✗ Timed out waiting for: ${pattern}`, true);
  }, timeoutMs);

  cdp = await connect(5000);
  const pages = await cdp.getPages();
  const page = pages.at(-1);
  if (!page) {
    throw new Error("No page found");
  }

  const sessionId = await cdp.attachToPage(page.targetId);

  const maybeMatch = (text) => {
    if (!text) return false;
    if (!text.includes(pattern)) return false;
    finish(0, `✓ matched: ${text}`);
    return true;
  };

  cleanupHandlers.push(
    cdp.on("Runtime.consoleAPICalled", (params, eventSessionId) => {
      if (finished || eventSessionId !== sessionId) return;
      const text = (params.args || []).map(remoteArgToText).join(" ").trim();
      maybeMatch(text);
    })
  );

  cleanupHandlers.push(
    cdp.on("Log.entryAdded", (params, eventSessionId) => {
      if (finished || eventSessionId !== sessionId) return;
      const text = params.entry?.text ? String(params.entry.text) : "";
      maybeMatch(text);
    })
  );

  await cdp.send("Runtime.enable", {}, sessionId);
  await cdp.send("Log.enable", {}, sessionId);
} catch (error) {
  if (error.message === "Missing pattern") {
    usage();
  }
  finish(1, `✗ ${error.message}`, true);
}
