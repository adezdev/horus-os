# Desktop

Related decision: [ADR-0010](../decisions/0010-desktop-stack.md).

The desktop is half of what makes Horus a daily driver. It aims to be
fast, keyboard-driven, and pleasant on a 1366×768 laptop panel with a
touchpad and touchscreen.

## Components

```
 apps (horus-ui) ──────┐ shared-memory window buffers + channel
                       ▼
 ┌───────────────── horus-compositor ─────────────────┐
 │ window manager (tiling + floating, workspaces)     │
 │ compositor (damage tracking, AVX2 blending → GPU)  │
 │ animations · theming · gestures · focus policy     │
 └───────┬─────────────────────────────┬──────────────┘
         │                             │
   input server                 display (kernel)
   (keyboard, touchpad,         v0.7: framebuffer
    touchscreen, gestures)      v0.10: Intel display engine
```

## Display protocol

A native, Wayland-*like* protocol over a channel (not Wayland itself):

- A client creates a **surface**, attaches **buffers** (shared memory
  objects), and **commits** with damage rectangles.
- The compositor sends **frame callbacks** paced to the display refresh,
  so clients only draw when a frame will actually be shown.
- **Input is routed by the compositor** to the focused surface, so
  clients can't snoop on each other's input.
- Popups, tooltips, drag-and-drop, clipboard, and screenshots are
  separate protocol objects, each permission-checked.
- **Scale factor** is part of the protocol from day one (1.0 on the
  internal panel, but an external HDMI monitor may differ).

## Rendering path

| Phase          | How frames reach the screen                                  |
| -------------- | ------------------------------------------------------------ |
| v0.7 to v0.9   | Software composition into a back buffer with AVX2, then copied to the Limine framebuffer. 1366×768×4 B ≈ 4.2 MB per frame, which is cheap with damage tracking. No vblank, so tearing is possible. |
| v0.10          | Native display engine: double/triple buffering with **atomic page flips on vblank**, hardware cursor plane, overlay planes for video. |
| Later          | GPU-accelerated composition and toolkit rendering on Xe-LP.  |

Every renderer sits behind one `Renderer` trait in `horus-ui` and the
compositor, so moving from CPU to GPU doesn't change apps.

**Frame budget target:** input event to updated pixels in **under
one frame (16.6 ms at 60 Hz)**; compositor work under 4 ms per frame.

## Window management

Modeled on Hyprland, which the owner uses today.

- **Dynamic tiling** (dwindle/master layouts) by default.
- **Floating** per window or per rule (dialogs, popups, utilities are
  floating automatically).
- **Workspaces**, switched by keyboard or a **3-finger horizontal
  swipe** on the touchpad.
- **Fullscreen** and maximized modes.
- **Window rules** (match by app ID/title → float, workspace, size, opacity).
- **Keyboard-first:** every action has a binding; bindings are
  configurable.
- **Animations:** spring-based open/close, move, and workspace slide
  that can be interrupted mid-way; all of them can be turned off.
- **Touchscreen:** tap to focus, drag title areas, edge swipes for
  launcher and overview.

## Toolkit: `horus-ui`

A native, retained-mode Rust GUI toolkit.

| Area        | Approach                                                       |
| ----------- | -------------------------------------------------------------- |
| Model       | Retained widget tree, declarative builder API, reactive state (signals) |
| Layout      | Flexbox and grid (e.g. via the `taffy` crate)                  |
| Text        | Shaping (`rustybuzz`) and rasterizing (`swash`/`fontdue`), subpixel positioning, font fallback |
| Rendering   | `Renderer` trait: CPU rasterizer first (e.g. `tiny-skia`), GPU later |
| Widgets     | Button, text input, list, tree, table, tabs, menu, scroll view, dialogs |
| Input       | Keyboard focus and navigation, pointer, touch, gestures, IME hooks |
| Accessibility | Accessibility tree in the design from the start; screen reader later |
| Theming     | All visuals come from the active theme (see below); no hard-coded colors |

Third-party crates listed above are candidates only. Their licenses
must be compatible with MIT OR Apache-2.0
([dependency policy](../development/workflow.md#dependency-licenses)).

## Theming

The desktop is **fully themeable**, like Omarchy themes.

- A theme is a directory: `theme.toml` plus optional assets
  (wallpapers, icons, fonts, cursors, sounds).
- `theme.toml` defines **design tokens**: palette, typography, spacing,
  corner radii, border widths, gaps, shadows, blur, opacity, and
  animation curves and durations.
- The compositor, toolkit, terminal, and shell prompt all read the same
  tokens, so one switch restyles everything.
- **Hot reload:** saving `theme.toml` restyles the running desktop
  without restarting apps.
- Ships with one default theme; the Egyptian identity (Eye of Horus logo,
  boot splash) belongs to branding, not to the theme, so themes stay free.

Example:

```toml
[meta]
name = "Horus Default"
variant = "dark"

[palette]
background = "#0f1115"
surface    = "#171a21"
text       = "#e6e6e6"
accent     = "#d4a63a"
error      = "#e05252"

[shape]
radius = 8
border = 2
gaps   = { inner = 6, outer = 10 }

[motion]
curve    = "spring(1, 180, 22)"
duration = "220ms"
```

## Desktop shell components

- **Status bar:** workspaces, focused window, clock, battery, network,
  volume, brightness.
- **Launcher:** fuzzy app and command search.
- **Notifications:** with history.
- **Lock screen:** used on lid close and idle.
- **Settings app:** display, input, network, sound, power, themes.
- **Terminal emulator:** runs `horus-shell`, GPU-ready text rendering.
