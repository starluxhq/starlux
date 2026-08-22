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

# KGlobalAccel takes a key as a Qt keycode, not a name. Letters, digits, Space
# and the function keys cover what anyone binds a launcher to; anything else
# falls back to picking the shortcut up at login.
qt_keycode() {
	total=0
	rest=$1
	while [ "$rest" != "${rest#*+}" ]; do
		part=${rest%%+*}
		rest=${rest#*+}
		case $part in
		Shift) total=$((total + 33554432)) ;;
		Ctrl | Control) total=$((total + 67108864)) ;;
		Alt) total=$((total + 134217728)) ;;
		Meta | Super) total=$((total + 268435456)) ;;
		*) return 1 ;;
		esac
	done
	case $rest in
	Space) total=$((total + 32)) ;;
	[A-Z0-9]) total=$((total + $(printf '%d' "'$rest"))) ;;
	[a-z]) total=$((total + $(printf '%d' "'$rest") - 32)) ;;
	F[1-9] | F1[0-2]) total=$((total + 16777264 + ${rest#F} - 1)) ;;
	*) return 1 ;;
	esac
	echo "$total"
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

# Writing the file is not enough: the running shortcut daemon has the old set in
# memory and only re-reads at login. Registering over D-Bus is what makes the key
# work now, and it persists to the same file in the same format.
key=$(qt_keycode "$KEY") || key=
if [ -n "$key" ] && command -v busctl >/dev/null 2>&1 &&
	busctl --user call org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel \
		doRegister as 4 "$ID" _launch "$NAME" "$NAME" >/dev/null 2>&1 &&
	busctl --user call org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel \
		setShortcut asaiu 4 "$ID" _launch "$NAME" "$NAME" 1 "$key" 2 >/dev/null 2>&1; then
	when="now"
else
	# Plasma before 6.5 runs the daemon as its own unit; from 6.5 KWin serves the
	# interface and this unit exits immediately, which is why it is the fallback.
	systemctl --user restart plasma-kglobalaccel.service 2>/dev/null || true
	when="after you log out and back in"
fi

cat <<EOF
bound $KEY to \`$binary --toggle\`, active $when

Nothing checked whether $KEY was already taken; if it does nothing, look for a
conflict under System Settings > Keyboard > Shortcuts, or rerun with
STARLUX_SHORTCUT set to something else. Meta+Space in particular is a common
launcher binding.

To undo: remove $apps/$ID and the [services][$ID] group from
\${XDG_CONFIG_HOME:-\$HOME/.config}/kglobalshortcutsrc.
EOF
