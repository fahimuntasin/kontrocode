/**
 * Terminal emulator. xterm.js + a Tauri shell command bridge. Phase 1:
 * each line typed + Enter runs `cmd_shell_run` and prints the result.
 * Multi-pane and PTY come in Phase 6.
 */

import { type Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import { Terminal as XTerm } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { shellRun } from "../lib/api";
import { ui } from "../lib/store";
import "xterm/css/xterm.css";

export const Terminal: Component = () => {
  let host: HTMLDivElement | undefined;
  let term: XTerm | undefined;
  let fit: FitAddon | undefined;
  const [hidden] = createSignal(false);

  onMount(() => {
    if (!host) return;
    term = new XTerm({
      fontFamily: '"JetBrains Mono", "Menlo", monospace',
      fontSize: 12,
      lineHeight: 1.4,
      theme: {
        background: "#0D0D1A",
        foreground: "#E8E8FF",
        cursor: "#3A3AFF",
        cursorAccent: "#0D0D1A",
        black: "#13132B",
        red: "#FF4466",
        green: "#00FFB2",
        yellow: "#FFB347",
        blue: "#3A3AFF",
        magenta: "#B83AFF",
        cyan: "#00FFB2",
        white: "#E8E8FF",
        brightBlack: "#44447A",
        brightRed: "#FF6B8A",
        brightGreen: "#5CFFCE",
        brightYellow: "#FFD085",
        brightBlue: "#7B7BFF",
        brightMagenta: "#D070FF",
        brightCyan: "#5CFFCE",
        brightWhite: "#FFFFFF",
      },
      cursorBlink: true,
      cursorStyle: "block",
      allowProposedApi: true,
      scrollback: 5000,
      convertEol: true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();
    // Re-fit on window resize.
    const ro = new ResizeObserver(() => {
      try {
        fit?.fit();
      } catch {
        /* ignore */
      }
    });
    ro.observe(host);

    // Greet.
    term.writeln("\x1b[36mKontroCode terminal\x1b[0m — Phase 1");
    term.writeln("Shell commands run inside the project root.");
    term.writeln("");

    let buffer = "";
    const prompt = () => term!.write("\x1b[34m$\x1b[0m ");
    prompt();

    term.onData(async (data) => {
      if (!term) return;
      for (const ch of data) {
        const code = ch.charCodeAt(0);
        if (code === 13) {
          // Enter
          term.write("\r\n");
          const cmd = buffer.trim();
          buffer = "";
          if (cmd.length > 0) {
            await runCommand(cmd);
          }
          prompt();
        } else if (code === 127) {
          // Backspace
          if (buffer.length > 0) {
            buffer = buffer.slice(0, -1);
            term.write("\b \b");
          }
        } else if (code === 3) {
          // Ctrl-C
          term.write("^C\r\n");
          buffer = "";
          prompt();
        } else if (code >= 32) {
          buffer += ch;
          term.write(ch);
        }
      }
    });

    onCleanup(() => {
      ro.disconnect();
      term?.dispose();
    });
  });

  async function runCommand(line: string): Promise<void> {
    if (!term) return;
    // Simple parser: split on spaces; quoted args come in Phase 6.
    const parts = line.split(/\s+/).filter(Boolean);
    const command = parts[0];
    if (!command) return;
    const args = parts.slice(1);
    if (command === "clear" || command === "cls") {
      term.clear();
      return;
    }
    try {
      const out = await shellRun(command, args);
      for (const ln of out.split("\n")) {
        term.writeln(ln);
      }
    } catch (e) {
      term.writeln(`\x1b[31m${String(e)}\x1b[0m`);
    }
  }

  return (
    <Show when={ui.terminalVisible() && !hidden()}>
      <div class="terminal" style={{ "border-top": "1px solid var(--border-subtle)", background: "var(--bg-void)", position: "relative", height: "100%", "min-height": 0 }}>
        <div class="section-header" style={{ "border-bottom": "none" }}>
          <span>Terminal</span>
          <button
            type="button"
            class="icon-button"
            aria-label="Hide terminal"
            title="Hide terminal (⌘`)"
            onClick={() => ui.toggleTerminal()}
          >
            ×
          </button>
        </div>
        <div ref={host} style={{ position: "absolute", inset: "24px 0 0 0", padding: "4px 8px" }} />
      </div>
    </Show>
  );
};
