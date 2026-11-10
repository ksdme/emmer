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
- **rodio** for notification sounds

## Configuration

Emmer reads `$XDG_CONFIG_HOME/emmer/config.kdl`, or the file given with
`--config`. Only the values that differ from the defaults need to be set. To
start from the defaults, with each option documented:

```sh
emmer default-config > ~/.config/emmer/config.kdl
```

Notifications show in the top right corner by default. To move them, set the
`anchor` block:

```kdl
anchor {
    horizontal "center"
    vertical "bottom"
}
```

## Pending

- Text alignment
- Markup text
- HiDPI
- Center the image
- Multiple stacks
- Rule-based theming
- Multiple monitors

**Tests**
- Clicking a notification should dismiss it and activate the app
