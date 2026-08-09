# Emmer

A Wayland notification daemon, inspired by [Sonner](https://sonner.emilkowal.ski/).

<p align="center" width="100%">
  <video src="https://github.com/user-attachments/assets/a842ff3a-3e77-4ee4-9f23-1353ccbddd66" width="80%" controls />
</p>

## Stack

- **Rust**
- **Wayland** (smithay-client-toolkit)
- **Cairo / Pango** for rendering
- **zbus** for the freedesktop Notifications D-Bus API
- **Tokio** for async

## Pending

**Current**
- Dismiss all button (bar stay until empty, hoverable clear, height/layout quirks)
- Dismiss goes too far down sometimes
- Configuration file fixes

**Later**
- Sounds and sound hints
- Scrolling spread
- Text alignment
- Font parsing
- Animation smoothing
- Better theming
- Default configuration file
- Markup text
- Cap frame rate
- HiDPI
- Center the image
- Multiple stacks
- Rule-based theming

**Tests**
- Clicking a notification should dismiss it and activate the app
