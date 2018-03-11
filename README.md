# sgstatus

Status monitor for Sway's status bar (`swaybar`).

## Features

sgstatus monitors `dbus` and `pulseaudio` to send status updates when the state 
or properties of the monitored interfaces changes. This is in contrast with 
other tiling window manager status tools, which usually monitor at 
pre-determined intervals, ex: checking if the volume has changed every 5 
seconds, etc.

sgstatus also (partially) implements the `StatusNotifierItem` protocol so that
it can send icons to the status bar.

It currently monitors the following:

* Battery level
* Network connectivity
* Volume (EXPERIMENTAL)

## Disclaimer

sgstatus is a work in progress. It doesn't do a lot of things at the moment. 

# Usage

## Enabling sgstatus as a Sway status command

sgstatus can be added as the status application by modifying the Sway 
configuration (typically `~/.config/sway/config`). Edit the `status_command`
line of the `bar` config block to the following:
```
status_command <path to sgstatus executable>
```

sgstatus will log to `stderr`. The following example demonstrates how to log the
output to a file:
```
status command <path to sgstatus executable> 2> sgstatus.log
```

## Choosing icon set in Sway

sgstatus uses common symbolic icons that are supported by various icon sets. 
Sway will do its best to locate the right icons. The `Adwaita` icon set is 
known to work out of the box with sgstatus. To configure it, edit the `bar` 
config block in the Sway config with the following line:
```
icon_theme Adwaita
```
