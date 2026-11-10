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

## Installation

Emmer is started on demand through D-Bus activation, like mako. Installing it
copies the binary, a D-Bus service file and a systemd user unit under `PREFIX`
(`~/.local` by default):

```sh
task build
task install                            # or: sudo task install PREFIX=/usr/local
systemctl --user daemon-reload
```

The first notification sent will start `emmer.service`.

Only one daemon can own `org.freedesktop.Notifications`. The D-Bus service file
in `~/.local/share` takes precedence over the ones in `/usr/share`, but it is
best to also mask the existing daemon so that it can't be started at all. For
mako:

```sh
systemctl --user mask --now mako.service
systemctl --user reload dbus-broker.service
```

The compositor needs to export `WAYLAND_DISPLAY` to the systemd user
environment; otherwise the service will not start. Most sessions do this
already. If yours doesn't, add something like this to its config:

```sh
systemctl --user import-environment WAYLAND_DISPLAY
```

To switch back to mako:

```sh
task uninstall
systemctl --user unmask mako.service
systemctl --user reload dbus-broker.service
```

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

To change the look of the notifications from an app, add a `rule` that matches
its name and mirrors the parts of the config to override. Like mako's criteria,
every rule that matches applies in order, so the later ones win. For now, only
the font and color of the title, and the colors and font of the action buttons
can be overridden.

```kdl
rule app-name="Slack" {
    notification {
        title {
            color "#e01e5a"
        }
    }
}
```

## Pending

- Text alignment
- Markup text
- HiDPI
- Center the image
- Multiple stacks
- Multiple monitors
- Momentum scroll on touchpads

**Tests**
- Clicking a notification should dismiss it and activate the app
