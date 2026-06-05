# Design

> The visual + UX language of KontroCode. Read this before touching the UI.

## Philosophy

> Not Cursor. Not VS Code. Not Zed. **KontroCode.**

KontroCode has its own visual identity. The rules below are non-negotiable.

| Rule | What it means | Why |
|------|---------------|-----|
| **Dark-first** | Deep void navy `#0D0D1A` background. Never grey. | Reduces eye strain; sets a serious, "tool for engineers" tone. |
| **Electric accent** | `#3A3AFF` blue for active states, focus, cursor. | High signal, low noise. Distinct from green/purple used by other tools. |
| **Minimal chrome** | No toolbar clutter. Code gets 80% of screen. | Information density is the feature. |
| **Panels slide, never modal** | Agent chat, research feed, memory are docks. Always accessible. | Interruption-free flow. |
| **Monospace everywhere code appears** | JetBrains Mono / Zed Mono for code, Inter for prose. | The code is the UI. |
| **Zed-level density** | Compact line height, 13px base, tight padding. | More code per square centimeter. |

## Theme tokens

```css
:root {
  /* Surfaces */
  --bg-void:        #0D0D1A;  /* deep void navy — primary background */
  --bg-surface:     #13132B;  /* panels, sidebar */
  --bg-elevated:    #1A1A38;  /* popovers, tooltips */
  --bg-overlay:     rgba(13, 13, 26, 0.85);

  /* Borders */
  --border-subtle:  #2A2A4E;
  --border-strong:  #3F3F70;

  /* Accents */
  --accent-primary:   #3A3AFF;  /* active, highlights, cursor */
  --accent-secondary: #00FFB2;  /* success, research complete */
  --accent-warning:   #FFB347;
  --accent-error:     #FF4466;  /* errors, deprecation warnings */

  /* Text */
  --text-primary:   #E8E8FF;
  --text-muted:     #6666AA;
  --text-faint:     #44447A;
  --text-on-accent: #FFFFFF;

  /* Typography */
  --font-code: 'JetBrains Mono', 'Zed Mono', 'Menlo', monospace;
  --font-ui:   'Inter', -apple-system, system-ui, sans-serif;

  /* Sizing */
  --font-size-base:   13px;
  --font-size-code:   13px;
  --line-height-code: 1.55;
  --radius-sm:        4px;
  --radius-md:        6px;
  --radius-lg:        10px;

  /* Motion */
  --transition-fast:   120ms cubic-bezier(0.2, 0, 0, 1);
  --transition-medium: 220ms cubic-bezier(0.2, 0, 0, 1);
  --transition-slow:   360ms cubic-bezier(0.2, 0, 0, 1);
}
```

> **Do not** introduce new colors. If you need a new state, extend the token list here in a PR and link the use case.

## Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ Title bar (custom-drawn, no OS chrome)                             │
│  ● ● ●  ~/projects/myapp                              ⌘K  ⚙  ───   │
├──────────┬──────────────────────────────────────────┬──────────────┤
│          │                                          │              │
│  File    │                                          │   Agent      │
│  tree    │            Monaco editor                 │   panel      │
│  + git   │            (full GPUI/Tauri webview)      │              │
│  status  │                                          │  ┌────────┐  │
│          │                                          │  │ chat   │  │
│          │                                          │  ├────────┤  │
│ 240px    │             flex / center                │  │research│  │
│          │                                          │  │ feed   │  │
│          │                                          │  ├────────┤  │
│          │                                          │  │memory  │  │
│          ├──────────────────────────────────────────┤  │ panel  │  │
│          │  Terminal (xterm.js)                     │  └────────┘  │
│          │  ~ 200px tall, resizable                 │   360px      │
├──────────┴──────────────────────────────────────────┴──────────────┤
│ Status bar:  main  •  claude-sonnet-4  •  0.4¢  •  ✓ 4 providers  │
└────────────────────────────────────────────────────────────────────┘
```

### Panel rules

- **Left panel (file tree):** collapsible via `Cmd+B`. Shows git status dots (red=untracked, green=modified, ●=staged). No folder icons in tree — text only.
- **Center (editor):** Monaco with a custom KontroCode theme that mirrors the CSS tokens. Breadcrumb at top. Minimap off by default (information density).
- **Right panel (agent):** three sub-panels stacked: chat (top, 60%), research feed (middle, 25%, accordion), memory (bottom, 15%, accordion). User can resize.
- **Bottom (terminal):** multi-pane. Toggle via `` Ctrl+` ``. Status of last command (✓ green or ✗ red) shown as 1px left border.
- **Status bar:** always visible, 24px tall. Shows active model, accumulated cost, provider health (4 dots), current branch.

### Density

- **Editor line height:** 1.55
- **Sidebar line height:** 1.4
- **Agent chat line height:** 1.6 (more breathing room — prose, not code)
- **No rounded corners on the editor.** Panels: 0px radius at the canvas, 6px on floating elements (command palette, tooltips).
- **No drop shadows.** Use 1px borders to denote elevation.

## Typography

- **Code:** JetBrains Mono 13px, 500 weight, ligatures on (`calt`).
- **UI:** Inter 13px, 400 regular / 500 medium / 600 semibold.
- **Headings in panels:** Inter 11px, 600, uppercase, `--text-muted`, letter-spacing 0.08em.
- **No** "code comments" in the UI that look like `# settings`. Use plain labels.

## Motion

- Panel slide-in: 220ms `cubic-bezier(0.2, 0, 0, 1)` — fast enough to feel instant.
- Agent streaming: code appears at the model's natural pace. No typewriter effect. No "thinking..." spinner. Instead, the research feed shows what is being looked up.
- Hover: 120ms, no transitions on color unless interactive.

## Iconography

- Lucide icons, 16px stroke 1.5. Monochrome — colored only when state is the meaning (e.g., red error).
- No icons in the file tree. Filename + git status dot only.
- The KontroCode logomark is a hexagonal "K" with the electric blue accent on the diagonal stroke. (See `apps/desktop/src-tauri/icons/`.)

## Accessibility

- All text on background passes WCAG AA (4.5:1 minimum). `--text-primary` on `--bg-void` is 13.4:1.
- Focus rings: 2px solid `--accent-primary` with 2px offset. Never remove.
- All interactive elements keyboard-reachable. Tab order: file tree → editor → agent panel.
- Screen reader labels on every icon-only button.
- High-contrast theme toggle in settings (Phase 7).
- No information conveyed by color alone — deprecation warnings always include the word "deprecated" in text.

## The 5 visual anti-patterns we never do

1. ❌ Grey background. Always navy or surface.
2. ❌ Green for primary accent. That's VS Code. We're blue.
3. ❌ Toolbar with 10 icons across the top. We have a command palette.
4. ❌ Modal dialogs for agent interaction. The agent panel is a dock.
5. ❌ Light theme by default. We may add one in Phase 7, but it will never be the default.

## Adding to this doc

If you change a token, change the value here *and* in `apps/desktop/src/styles/tokens.css`. Both must stay in sync. The CI check `pnpm lint:tokens` will fail if they diverge.
