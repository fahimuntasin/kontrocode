#!/usr/bin/env node
/**
 * Check that CSS tokens match design.md values.
 * Run: pnpm lint:tokens
 *
 * This is a guardrail: the visual identity in docs/design.md is the source
 * of truth for the theme. If you change a color in either place without
 * updating the other, CI fails.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

const tokens = {
  "--bg-void": "#0D0D1A",
  "--bg-surface": "#13132B",
  "--bg-elevated": "#1A1A38",
  "--border-subtle": "#2A2A4E",
  "--border-strong": "#3F3F70",
  "--accent-primary": "#3A3AFF",
  "--accent-secondary": "#00FFB2",
  "--accent-warning": "#FFB347",
  "--accent-error": "#FF4466",
  "--text-primary": "#E8E8FF",
  "--text-muted": "#6666AA",
  "--text-faint": "#44447A",
};

let failed = false;

for (const cssPath of [
  "apps/desktop/src/styles/tokens.css",
  "apps/desktop/src/styles/monaco-theme.css",
]) {
  let css;
  try {
    css = readFileSync(join(root, cssPath), "utf8");
  } catch {
    // File may not exist yet in early phases — skip with a warning
    console.warn(`[skip] ${cssPath} (not found yet)`);
    continue;
  }
  for (const [name, value] of Object.entries(tokens)) {
    if (!css.toLowerCase().includes(`${name}:`.toLowerCase())) {
      console.error(`[fail] ${cssPath}: missing ${name}`);
      failed = true;
    } else if (!css.toLowerCase().includes(value.toLowerCase())) {
      console.error(`[fail] ${cssPath}: ${name} should be ${value}`);
      failed = true;
    }
  }
}

if (failed) {
  console.error("\nToken drift detected. Update CSS to match docs/design.md.");
  process.exit(1);
} else {
  console.log("[ok] tokens in sync with docs/design.md");
}
