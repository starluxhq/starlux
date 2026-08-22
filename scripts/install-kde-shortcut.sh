#!/bin/sh
# Bind a KDE global shortcut to `starlux --toggle`.
#
# Wayland does not let an application grab keys for itself, so on Plasma the
# hotkey has to be registered with the desktop rather than by Starlux. Plasma 6
# stores a command shortcut as a hidden .desktop file plus an entry in
# kglobalshortcutsrc, which is what this writes.
#
#   ./scripts/install-kde-shortcut.sh [/path/to/starlux]
#   STARLUX_SHORTCUT="Ctrl+Alt+Space" ./scripts/install-kde-shortcut.sh
set -e

ID=net.local.starlux.desktop
NAME="Toggle Starlux"
KEY=${STARLUX_SHORTCUT:-Meta+Space}

die() {
	echo "install-kde-shortcut: $1" >&2
	exit 1
}

kwriteconfig=$(command -v kwriteconfig6 || command -v kwriteconfig5 || true)
[ -n "$kwriteconfig" ] || die "no kwriteconfig6 found. Is this a KDE Plasma session?"

binary=${1:-$(command -v starlux || true)}
if [ -z "$binary" ]; then
	repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
	for candidate in "$repo/src-tauri/target/release/starlux" "$repo/src-tauri/target/debug/starlux"; do
		[ -x "$candidate" ] && binary=$candidate && break
	done
fi
[ -n "$binary" ] || die "could not find the starlux binary. Pass its path as an argument."
[ -x "$binary" ] || die "\`$binary\` is not executable."

apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
mkdir -p "$apps"
cat >"$apps/$ID" <<EOF
[Desktop Entry]
Type=Application
Name=$NAME
Exec="$binary" --toggle
NoDisplay=true
StartupNotify=false
X-KDE-GlobalAccel-CommandShortcut=true
EOF

command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 --noincremental >/dev/null 2>&1 || true

# Same two pieces System Settings writes for a custom command shortcut: the
# hidden .desktop above, and the key against its name here.
"$kwriteconfig" --file kglobalshortcutsrc --group services --group "$ID" \
	--key _launch "$KEY"

# The daemon reads the file once at startup, so a live session needs a restart
# to pick this up.
if ! systemctl --user restart plasma-kglobalaccel.service 2>/dev/null; then
	echo "note: could not restart the shortcut daemon — log out and back in to activate."
fi

cat <<EOF
bound $KEY to \`$binary --toggle\`

Nothing checked whether $KEY was already taken; if it does nothing, look for a
conflict under System Settings > Keyboard > Shortcuts, or rerun with
STARLUX_SHORTCUT set to something else.

To undo: remove $apps/$ID and the [services][$ID] group from
\${XDG_CONFIG_HOME:-\$HOME/.config}/kglobalshortcutsrc.
EOF
