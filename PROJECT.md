# EEG Pen — Project Specification

## 1. Purpose

A lightweight desktop application for **on-screen review and annotation of EEG data** during clinical consultations, designed for use on two displays simultaneously (clinician display + parent-facing display). The app does **not** replace the EEG acquisition/reporting software — it sits on top of it as a **screen-annotation and review overlay**, letting the clinician zoom, magnify, highlight, and point at regions of interest live, in front of parents.

> **Assumption to confirm:** V1 treats EEG data as **on-screen visual content** (live capture from the existing EEG viewer, a screenshot, an exported image, or a PDF page) rather than parsing raw waveform files (e.g. EDF/BDF) directly. This keeps V1 small, fast, and viewer-agnostic — it works no matter which EEG acquisition software is used. If you actually need native EDF waveform rendering, that changes the architecture significantly and should be scoped as a separate, larger feature (flagged below in V2/V3 considerations).

## 2. Goals

- Fast, distraction-free annotation tools for live parent consultations
- Small installer, minimal resource footprint, instant startup
- Identical experience on Windows and macOS from one codebase
- Keyboard-shortcut-driven for speed during live review
- Dual-monitor aware (control on clinician screen, clean view on parent screen)
- Zero patient data leaves the device in V1 (fully offline)

## 3. Non-Goals (V1)

- No EEG signal processing, filtering, or raw waveform decoding
- No AI/model integration (planned for V2)
- No cloud sync, accounts, or telemetry
- No editing/annotating of patient records or EHR integration

## 4. Target Platforms & Priority

| Platform | Priority | Status |
|---|---|---|
| Windows 10/11 (x64) | **P0 — build first** | V1 target |
| macOS 12+ (Apple Silicon + Intel) | P1 — parallel-planned | V1.x target, same codebase |

Both platforms are developed from a **single shared codebase in one repository**, with platform-specific build targets — not two separate projects. See §7.

## 5. Recommended Tech Stack

Given your priorities — small install size, native performance, cross-platform from one codebase, and your existing comfort with **Tauri** (from EdgeDictate/DictateAnywhere) — this app is a strong fit for the same stack:

| Layer | Choice | Why |
|---|---|---|
| App shell | **Tauri 2.x** (Rust core) | ~3–10 MB installers vs. 80–150 MB for Electron; native webview (WebView2 on Windows, WKWebView on macOS) so no bundled Chromium |
| Frontend UI | **Svelte + TypeScript** | Small bundle, fast, matches your EdgeDictate stack — reusable knowledge/tooling |
| Canvas/annotation layer | **Konva.js** or raw HTML5 Canvas | Needed for zoom, magnifier loupe, circle/box overlays, arrows — Konva gives you shape objects, hit-testing, and layering for free |
| Screen/window capture (for magnify & future screenshot feature) | Tauri's native window/screen APIs + `scap` or platform screencapture crates | Rust-native, avoids Electron-style capture hacks |
| Multi-monitor handling | Tauri multiwindow API | Native window placement per display |
| Packaging | Tauri bundler (MSI/NSIS for Windows, `.dmg`/`.app` for macOS) | One config, both targets |
| Global shortcuts | Tauri `global-shortcut` plugin | System-wide hotkeys even when app isn't focused, useful during live review |

**Estimated installer size target: under 15 MB** (Tauri baseline is typically 3–8 MB before app logic).

## 6. Core V1 Features

### 6.1 Zoom / Magnify Tool
- Click-and-drag or hover-triggered **magnifier loupe**: creates a square/rectangular border around a region and displays a zoomed-in view of it (like a digital magnifying glass), without altering the underlying screen content.
- Adjustable zoom level (e.g. 1.5x–6x), adjustable loupe size.
- Shortcut: `M` to toggle magnifier mode, `[` / `]` to decrease/increase zoom level, mouse wheel as alternate zoom control.

### 6.2 Region Highlight (Box)
- Draw a rectangular highlight box around an area of interest (e.g. a spike-wave discharge, an artifact).
- Adjustable border color, thickness, and optional semi-transparent fill.
- Persist until manually cleared or auto-clear after N seconds (configurable).
- Shortcut: `B` to activate box tool, click-drag to draw, `Esc` to cancel current draw.

### 6.3 Circle Highlight
- Draw a circular/elliptical highlight, same styling options as box.
- Shortcut: `C` to activate circle tool.

### 6.4 Colored Pointing Arrows
- Click to drop an arrow pointing at a specific location; drag to set direction/length.
- Multiple preset colors (e.g. red/yellow/green/blue) mapped to number keys for instant selection.
- Shortcut: `A` to activate arrow tool, `1`–`4` to pick color before/while placing.

### 6.5 Annotation Management
- All annotations (boxes, circles, arrows) are **temporary overlay objects**, not permanent edits to underlying EEG data/images — they exist only in the review session.
- `Ctrl/Cmd + Z` — undo last annotation
- `Delete` — clear all annotations
- `Space` (hold) — temporarily hide all annotations to view clean EEG underneath

### 6.6 Corner Control Panel (UI)
- Small, collapsible **floating control panel anchored to a screen corner** (default: bottom-right, user-configurable to any corner).
- Panel contains: tool selector icons (zoom, box, circle, arrow), color swatches, clear/undo buttons, a "send to parent display" toggle.
- Panel auto-fades to low opacity when not in use (mouse-away), fully opaque on hover — keeps the view uncluttered during consultation.
- Panel is **only shown on the clinician's display**, never on the parent-facing display/window.

### 6.7 Dual-Display Mode
- App detects connected displays on launch.
- Clinician window: full controls + annotation tools.
- Parent-facing window: mirrors the annotated EEG view in real time, **with controls hidden**, borderless/fullscreen option for a clean presentation.
- Toggle to swap which display is "parent-facing" without restarting.

### 6.8 Session Basics
- Load a static image/PDF page or capture a live region of the screen (from whatever EEG software is already open) as the working canvas.
- No patient identifiers stored or required by this app in V1 — it is a visual annotation layer only.

## 7. Project Structure (Single-Folder, Simultaneous Windows/macOS Development)

```
eeg-pen/
├── PROJECT.md
├── src-tauri/                 # Rust core (shared, compiles per-target)
│   ├── src/
│   │   ├── main.rs
│   │   ├── capture.rs         # screen/window capture logic
│   │   ├── shortcuts.rs       # global hotkey registration
│   │   ├── window_manager.rs  # multi-display window handling
│   │   └── commands.rs        # Tauri command bridge to frontend
│   ├── Cargo.toml
│   ├── tauri.conf.json        # base config
│   ├── tauri.windows.conf.json # Windows-specific overrides (icons, MSI/NSIS settings)
│   └── tauri.macos.conf.json  # macOS-specific overrides (entitlements, .icns, signing)
├── src/                        # Svelte + TS frontend (100% shared, no platform branching)
│   ├── lib/
│   │   ├── components/
│   │   │   ├── ControlPanel.svelte
│   │   │   ├── CanvasLayer.svelte
│   │   │   ├── Magnifier.svelte
│   │   │   ├── ShapeTool.svelte
│   │   │   └── ArrowTool.svelte
│   │   ├── stores/            # annotation state, tool state, display state
│   │   └── utils/
│   ├── App.svelte
│   └── main.ts
├── build/
│   ├── windows/                # generated installers land here
│   └── macos/                  # generated .dmg/.app land here
├── docs/
│   └── shortcuts-reference.md
└── package.json
```

**Build commands (both from the same folder):**
```bash
npm run tauri build -- --target x86_64-pc-windows-msvc   # Windows
npm run tauri build -- --target universal-apple-darwin   # macOS (once on Mac hardware or CI)
```

> Note: macOS builds require macOS build hardware or a CI runner (e.g. GitHub Actions `macos-latest`) — Tauri cannot cross-compile a signed `.app`/`.dmg` from Windows. Plan for either (a) occasional access to a Mac / Mac VM, or (b) a CI pipeline that builds the macOS target on push, even while day-to-day development stays on Windows. This is the practical meaning of "simultaneous development" here: one shared codebase, with macOS packaging validated via CI rather than local Mac hardware.

## 8. Keyboard Shortcuts (V1 Draft)

| Shortcut | Action |
|---|---|
| `M` | Toggle magnifier tool |
| `B` | Box highlight tool |
| `C` | Circle highlight tool |
| `A` | Arrow tool |
| `1`–`4` | Select annotation color |
| `[` / `]` | Decrease / increase zoom |
| `Ctrl/Cmd+Z` | Undo last annotation |
| `Delete` | Clear all annotations |
| `Space` (hold) | Hide annotations temporarily |
| `Ctrl/Cmd+D` | Swap clinician/parent display |
| `Esc` | Cancel current tool action |

All shortcuts remap-able via a settings panel (stored locally).

## 9. Non-Functional Requirements

- **Startup time:** under 1 second on typical clinic hardware
- **Installer size:** target under 15 MB (Windows), comparable for macOS
- **Memory footprint:** under 150 MB during active use
- **Offline-first:** fully functional with no internet connection (hard requirement for V1)
- **Data handling:** no patient-identifying data collected, transmitted, or logged by the app itself in V1
- **Code signing:** plan for a Windows code-signing certificate and an Apple Developer ID (for macOS Gatekeeper) before distribution outside your own machines — unsigned builds will trigger security warnings on both OSes

## 10. Version 2 — Planned Future Features

These are explicitly **out of scope for V1** and scoped here so they don't creep into the initial build:

1. **Screenshot capture pipeline**
   - One-shortcut capture of the current EEG view (with or without annotations) to a local buffer.
2. **AI-assisted analysis of captured screenshots**
   - Send captured screenshots to an analysis model, with **two modes**:
     - **Offline/local model** (e.g. a locally-hosted vision model) — no data leaves the device, preferred default for patient safety and privacy.
     - **Online model** (cloud API) — opt-in only, with an explicit consent/warning step before any transmission, given this is patient health data (PHI). This needs a documented data-handling review (what's sent, retention policy of the provider, DHA/HIPAA-equivalent compliance) before it's enabled — flag for legal/compliance review, not just an engineering decision.
   - Structured output showing model observations alongside the original image, clearly labeled as **assistive/non-diagnostic**.
3. **Session recording / export**
   - Save an annotated review session (image + overlays) as a PDF or image file for the patient chart, with patient/session metadata entered manually by the clinician (kept separate from the AI pipeline).
4. **Annotation presets & templates**
   - Save common annotation patterns (e.g. "typical spike-wave callout") as reusable one-click templates.
5. **Native EDF/waveform support (larger scope item)**
   - If needed later: direct parsing and rendering of raw EEG waveform files rather than screen capture, enabling programmatic zoom/scroll through actual signal data rather than visual-only zoom. This is a materially larger engineering effort and should be scoped as its own mini-project if pursued.
6. **Multi-language UI**
   - For clinics with multilingual parent populations.
7. **Cloud-optional session sync**
   - Encrypted, opt-in sync of session exports across the clinician's own devices (not a general cloud backend).
8. **Audit log**
   - Local log of when AI analysis was invoked and on what data, to support compliance documentation — only relevant once V2's AI features exist.

## 11. Development Approach

- Built via **vibe coding on Google Antigravity**, iterating directly against this spec.
- Suggested build order for V1:
  1. Tauri shell + basic window (Windows target) boots and shows a blank canvas
  2. Screen/region capture working, displayed on canvas
  3. Magnifier tool
  4. Box + circle highlight tools
  5. Arrow tool + color shortcuts
  6. Corner control panel UI + auto-fade behavior
  7. Dual-display detection and parent-facing mirror window
  8. Global shortcut wiring + settings panel for remapping
  9. Windows installer packaging + signing
  10. macOS CI build validation (via GitHub Actions `macos-latest`), packaging + signing
- Each numbered step is a natural checkpoint to test standalone before moving to the next.

## 12. Open Questions to Resolve Before/During Build

- Confirm the "screen capture of existing EEG viewer" assumption in §1 is correct, vs. needing native waveform file support.
- Confirm target Windows EEG software so capture logic can be tuned to its window (e.g. avoiding capturing other overlapping windows).
- Confirm whether any patient identifiers will ever touch this app (even in V2 exports) — this determines whether DHA/health-data compliance requirements apply to the app itself, not just the AI pipeline.
