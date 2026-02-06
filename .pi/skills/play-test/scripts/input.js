#!/usr/bin/env node

/**
 * Send keyboard/mouse input to the game via CDP Input domain.
 *
 * Usage:
 *   input.js key <key>               Press and release a key (e.g. "KeyW", "Space", "ShiftLeft", "KeyJ")
 *   input.js key <key> --hold <ms>   Hold key for N ms before releasing
 *   input.js keys <k1> <k2> ...      Press multiple keys in sequence with 150ms gap
 *   input.js click <x> <y>           Click at viewport coordinates
 *   input.js type <text>             Type text character by character
 *
 * Key names use the DOM KeyboardEvent.code format:
 *   Letters: KeyA-KeyZ
 *   Digits: Digit0-Digit9
 *   Arrows: ArrowUp, ArrowDown, ArrowLeft, ArrowRight
 *   Special: Space, Enter, ShiftLeft, ShiftRight, Tab, Escape, Backspace
 */

import { fileURLToPath } from "url";
import { dirname, resolve } from "path";
import { homedir } from "os";

const cdpPath = resolve(
  homedir(),
  ".pi/agent/git/github.com/mitsuhiko/agent-stuff/skills/web-browser/scripts/cdp.js"
);
const { connect } = await import(cdpPath);

const DELAY_BETWEEN_KEYS = 150;

// Map DOM code values to CDP key descriptors
const KEY_MAP = {
  KeyA: { key: "a", code: "KeyA", windowsVirtualKeyCode: 65 },
  KeyB: { key: "b", code: "KeyB", windowsVirtualKeyCode: 66 },
  KeyC: { key: "c", code: "KeyC", windowsVirtualKeyCode: 67 },
  KeyD: { key: "d", code: "KeyD", windowsVirtualKeyCode: 68 },
  KeyE: { key: "e", code: "KeyE", windowsVirtualKeyCode: 69 },
  KeyF: { key: "f", code: "KeyF", windowsVirtualKeyCode: 70 },
  KeyG: { key: "g", code: "KeyG", windowsVirtualKeyCode: 71 },
  KeyH: { key: "h", code: "KeyH", windowsVirtualKeyCode: 72 },
  KeyI: { key: "i", code: "KeyI", windowsVirtualKeyCode: 73 },
  KeyJ: { key: "j", code: "KeyJ", windowsVirtualKeyCode: 74 },
  KeyK: { key: "k", code: "KeyK", windowsVirtualKeyCode: 75 },
  KeyL: { key: "l", code: "KeyL", windowsVirtualKeyCode: 76 },
  KeyM: { key: "m", code: "KeyM", windowsVirtualKeyCode: 77 },
  KeyN: { key: "n", code: "KeyN", windowsVirtualKeyCode: 78 },
  KeyO: { key: "o", code: "KeyO", windowsVirtualKeyCode: 79 },
  KeyP: { key: "p", code: "KeyP", windowsVirtualKeyCode: 80 },
  KeyQ: { key: "q", code: "KeyQ", windowsVirtualKeyCode: 81 },
  KeyR: { key: "r", code: "KeyR", windowsVirtualKeyCode: 82 },
  KeyS: { key: "s", code: "KeyS", windowsVirtualKeyCode: 83 },
  KeyT: { key: "t", code: "KeyT", windowsVirtualKeyCode: 84 },
  KeyU: { key: "u", code: "KeyU", windowsVirtualKeyCode: 85 },
  KeyV: { key: "v", code: "KeyV", windowsVirtualKeyCode: 86 },
  KeyW: { key: "w", code: "KeyW", windowsVirtualKeyCode: 87 },
  KeyX: { key: "x", code: "KeyX", windowsVirtualKeyCode: 88 },
  KeyY: { key: "y", code: "KeyY", windowsVirtualKeyCode: 89 },
  KeyZ: { key: "z", code: "KeyZ", windowsVirtualKeyCode: 90 },
  Digit0: { key: "0", code: "Digit0", windowsVirtualKeyCode: 48 },
  Digit1: { key: "1", code: "Digit1", windowsVirtualKeyCode: 49 },
  Digit2: { key: "2", code: "Digit2", windowsVirtualKeyCode: 50 },
  Digit3: { key: "3", code: "Digit3", windowsVirtualKeyCode: 51 },
  Digit4: { key: "4", code: "Digit4", windowsVirtualKeyCode: 52 },
  Digit5: { key: "5", code: "Digit5", windowsVirtualKeyCode: 53 },
  Digit6: { key: "6", code: "Digit6", windowsVirtualKeyCode: 54 },
  Digit7: { key: "7", code: "Digit7", windowsVirtualKeyCode: 55 },
  Digit8: { key: "8", code: "Digit8", windowsVirtualKeyCode: 56 },
  Digit9: { key: "9", code: "Digit9", windowsVirtualKeyCode: 57 },
  Space: { key: " ", code: "Space", windowsVirtualKeyCode: 32 },
  Enter: { key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 },
  Escape: { key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 },
  Tab: { key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 },
  Backspace: { key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8 },
  ShiftLeft: { key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16, location: 1 },
  ShiftRight: { key: "Shift", code: "ShiftRight", windowsVirtualKeyCode: 16, location: 2 },
  ArrowUp: { key: "ArrowUp", code: "ArrowUp", windowsVirtualKeyCode: 38 },
  ArrowDown: { key: "ArrowDown", code: "ArrowDown", windowsVirtualKeyCode: 40 },
  ArrowLeft: { key: "ArrowLeft", code: "ArrowLeft", windowsVirtualKeyCode: 37 },
  ArrowRight: { key: "ArrowRight", code: "ArrowRight", windowsVirtualKeyCode: 39 },
};

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function pressKey(cdp, sessionId, keyCode, holdMs = 50) {
  const keyInfo = KEY_MAP[keyCode];
  if (!keyInfo) {
    console.error(`Unknown key code: ${keyCode}`);
    console.error(`Available keys: ${Object.keys(KEY_MAP).join(", ")}`);
    process.exit(1);
  }

  const baseParams = {
    key: keyInfo.key,
    code: keyInfo.code,
    windowsVirtualKeyCode: keyInfo.windowsVirtualKeyCode,
    nativeVirtualKeyCode: keyInfo.windowsVirtualKeyCode,
    ...(keyInfo.location !== undefined ? { location: keyInfo.location } : {}),
  };

  await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", ...baseParams }, sessionId);
  await sleep(holdMs);
  await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", ...baseParams }, sessionId);
}

async function clickAt(cdp, sessionId, x, y) {
  const params = { x, y, button: "left", clickCount: 1 };
  await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...params }, sessionId);
  await sleep(50);
  await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...params }, sessionId);
}

async function typeText(cdp, sessionId, text) {
  for (const char of text) {
    // Find the key code for this character
    const upper = char.toUpperCase();
    const keyCode = `Key${upper}`;
    if (KEY_MAP[keyCode]) {
      await pressKey(cdp, sessionId, keyCode, 30);
    } else {
      // Use insertText for characters we don't have key codes for
      await cdp.send("Input.insertText", { text: char }, sessionId);
    }
    await sleep(50);
  }
}

// --- Main ---
const args = process.argv.slice(2);
const command = args[0];

if (!command) {
  console.error("Usage: input.js <key|keys|click|type> <args...>");
  process.exit(1);
}

const globalTimeout = setTimeout(() => {
  console.error("✗ Timeout (30s)");
  process.exit(1);
}, 30000);

try {
  const cdp = await connect(5000);
  const pages = await cdp.getPages();
  const page = pages.at(-1);
  if (!page) {
    console.error("✗ No page found");
    process.exit(1);
  }
  const sessionId = await cdp.attachToPage(page.targetId);

  // Ensure the game canvas has focus so key events reach winit/Bevy
  await cdp.evaluate(
    sessionId,
    '(function() { var c = document.querySelector("canvas"); if (c) { c.tabIndex = 0; c.focus(); } })()'
  );

  switch (command) {
    case "key": {
      const keyCode = args[1];
      const holdIdx = args.indexOf("--hold");
      const holdMs = holdIdx !== -1 ? parseInt(args[holdIdx + 1], 10) : 50;
      if (!keyCode) {
        console.error("Usage: input.js key <KeyCode> [--hold <ms>]");
        process.exit(1);
      }
      await pressKey(cdp, sessionId, keyCode, holdMs);
      console.log(`✓ Pressed ${keyCode}`);
      break;
    }

    case "keys": {
      const keys = args.slice(1).filter((k) => !k.startsWith("--"));
      const delayIdx = args.indexOf("--delay");
      const delay = delayIdx !== -1 ? parseInt(args[delayIdx + 1], 10) : DELAY_BETWEEN_KEYS;
      for (const keyCode of keys) {
        await pressKey(cdp, sessionId, keyCode, 50);
        await sleep(delay);
      }
      console.log(`✓ Pressed ${keys.join(", ")}`);
      break;
    }

    case "click": {
      const x = parseFloat(args[1]);
      const y = parseFloat(args[2]);
      if (isNaN(x) || isNaN(y)) {
        console.error("Usage: input.js click <x> <y>");
        process.exit(1);
      }
      await clickAt(cdp, sessionId, x, y);
      console.log(`✓ Clicked (${x}, ${y})`);
      break;
    }

    case "type": {
      const text = args.slice(1).join(" ");
      if (!text) {
        console.error("Usage: input.js type <text>");
        process.exit(1);
      }
      await typeText(cdp, sessionId, text);
      console.log(`✓ Typed "${text}"`);
      break;
    }

    default:
      console.error(`Unknown command: ${command}`);
      process.exit(1);
  }

  cdp.close();
} catch (e) {
  console.error(`✗ ${e.message}`);
  process.exit(1);
} finally {
  clearTimeout(globalTimeout);
}
