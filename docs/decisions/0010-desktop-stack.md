# ADR-0010: Desktop: tiling + floating compositor, staged graphics, own toolkit, full theming

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

A modern desktop is a primary goal. The owner uses Hyprland on Omarchy today. The laptop panel is 1366×768 with Intel Xe-LP graphics, a touchpad, and a touchscreen.

## Decision

- **Window management:** dynamic tiling with floating windows, workspaces, gestures, and animations, modeled on Hyprland.
- **Graphics:** Limine/GOP framebuffer with an AVX2 software compositor first, then a native Intel display/GPU driver (v0.10).
- **Toolkit:** own retained-mode Rust toolkit, `horus-ui`, behind a `Renderer` trait.
- **Theming:** fully themeable via token-based `theme.toml` with hot reload.

See [desktop.md](../architecture/desktop.md).

## Consequences

- The desktop can be built long before the GPU driver exists.
- Software composition at 1366×768 is cheap but can tear until page flipping exists.
- Own toolkit means a consistent look and no porting, but a large amount of work.

## Alternatives considered

- Pure tiling, floating-only, scrolling tiling.
- Software rendering forever; full GPU acceleration early.
- Porting Slint/iced; immediate-mode UI.
