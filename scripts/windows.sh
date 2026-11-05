#!/usr/bin/env bash
# Build Arcanaeum for Windows and test it on Linux, with the Wine inside a Proton build.
#
#   scripts/windows.sh doctor    what is installed and what is missing
#   scripts/windows.sh setup     fetch what is missing (no root) and make the Windows prefix
#   scripts/windows.sh test      cargo test, as Windows binaries, run under Wine   [cargo test args...]
#   scripts/windows.sh build     a Windows arcanaeum.exe in target/x86_64-pc-windows-gnu/debug
#   scripts/windows.sh run       run that exe under Wine, in the throwaway prefix  [arcanaeum args...]
#   scripts/windows.sh clean     delete target/windows (the prefix and the fetched toolchain)
#
# The Makefile's windows-* targets call this. docs/windows-testing.md says what it does and why.
#
# Where things are is found, or set in the environment, or in scripts/windows.local.sh (ignored by
# git, sourced here if it exists):
#   PROTONPATH   a Proton build (a GE-Proton folder, holding files/bin/wine); found under Steam's
#                compatibilitytools.d folders if not set
#   WINE         a wine to use instead of Proton's
#   MINGW_BIN    a folder with x86_64-w64-mingw32-gcc, if it isn't on PATH
#   WINEPREFIX   the Windows prefix; target/windows/prefix if not set. Never point it at a real
#                game's prefix: the tests write config and Plugins.txt under its Windows user.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
target=x86_64-pc-windows-gnu
work=${WINDOWS_WORK:-$root/target/windows}

# shellcheck disable=SC1091
[ -f "$root/scripts/windows.local.sh" ] && . "$root/scripts/windows.local.sh"

say() { printf '%s\n' "$*" >&2; }
die() { say "windows: $*"; exit 1; }

find_proton() {
	if [ -n "${PROTONPATH:-}" ]; then
		echo "$PROTONPATH"
		return
	fi
	local d
	for d in "$HOME"/.local/share/Steam/compatibilitytools.d/*/ "$HOME"/.steam/root/compatibilitytools.d/*/ \
		/usr/share/steam/compatibilitytools.d/*/; do
		[ -x "${d}files/bin/wine" ] && { echo "${d%/}"; return; }
	done
	return 1
}

# The wine to run Windows programs with: WINE, else Proton's, else one on PATH.
find_wine() {
	if [ -n "${WINE:-}" ]; then
		echo "$WINE"
		return
	fi
	local p
	if p=$(find_proton) && [ -x "$p/files/bin/wine" ]; then
		echo "$p/files/bin/wine"
		return
	fi
	command -v wine || return 1
}

mingw_dir() {
	if [ -n "${MINGW_BIN:-}" ]; then
		echo "$MINGW_BIN"
	elif [ -d "$work/mingw/usr/bin" ]; then
		echo "$work/mingw/usr/bin"
	fi
}

# What cargo and Wine need: the linker, the runner, a prefix of our own, quiet Wine.
use_env() {
	local m
	m=$(mingw_dir)
	[ -z "$m" ] || PATH="$m:$PATH"
	export PATH
	command -v x86_64-w64-mingw32-gcc >/dev/null || die "no mingw-w64 gcc: run scripts/windows.sh setup"
	WINE=$(find_wine) || die "no Wine: set PROTONPATH to a GE-Proton folder, or install wine"
	export WINE
	export WINEPREFIX=${WINEPREFIX:-$work/prefix}
	export WINEDEBUG=${WINEDEBUG:--all}
	# No Mono or Gecko prompts: the app needs neither.
	export WINEDLLOVERRIDES="${WINEDLLOVERRIDES:+$WINEDLLOVERRIDES;}mscoree,mshtml="
	export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
	export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER="$root/scripts/windows.sh runner"
}

make_prefix() {
	if [ ! -d "$WINEPREFIX/drive_c" ]; then
		say "windows: making the Windows prefix $WINEPREFIX (once)"
		mkdir -p "$WINEPREFIX"
		"$WINE" wineboot -i >/dev/null 2>&1 || die "Wine couldn't make a prefix"
	fi
}

doctor() {
	local ok=0 p m
	if rustup target list --installed 2>/dev/null | grep -qx "$target"; then
		say "ok       rust target $target"
	else
		say "MISSING  rust target $target   (rustup target add $target)"
		ok=1
	fi
	m=$(mingw_dir)
	if PATH="${m:+$m:}$PATH" command -v x86_64-w64-mingw32-gcc >/dev/null; then
		say "ok       mingw-w64 gcc (${m:-on PATH})"
	else
		say "MISSING  mingw-w64 gcc   (sudo apt install gcc-mingw-w64-x86-64-posix, or: scripts/windows.sh setup)"
		ok=1
	fi
	if p=$(find_wine); then
		say "ok       wine: $p ($("$p" --version 2>/dev/null))"
	else
		say "MISSING  wine   (point PROTONPATH at a GE-Proton folder, or install wine)"
		ok=1
	fi
	return $ok
}

# mingw-w64 without root, from the distribution's packages unpacked under target/windows. Debian
# and Ubuntu only; elsewhere install mingw-w64 with the system's package manager.
fetch_mingw() {
	command -v x86_64-w64-mingw32-gcc >/dev/null && return
	[ -z "$(mingw_dir)" ] || return 0
	command -v apt-get >/dev/null || die "no mingw-w64 gcc, and this isn't Debian or Ubuntu: install mingw-w64 with your package manager"
	say "windows: fetching mingw-w64 (no root: unpacked under $work/mingw)"
	mkdir -p "$work/debs" "$work/mingw"
	(
		cd "$work/debs"
		apt-get download gcc-mingw-w64-x86-64-posix gcc-mingw-w64-x86-64-posix-runtime gcc-mingw-w64-base \
			binutils-mingw-w64-x86-64 mingw-w64-x86-64-dev mingw-w64-common >&2
	)
	local deb
	for deb in "$work"/debs/*.deb; do dpkg -x "$deb" "$work/mingw"; done
	# The package's postinst points the plain name at the posix-threads gcc; do it by hand.
	ln -sf x86_64-w64-mingw32-gcc-posix "$work/mingw/usr/bin/x86_64-w64-mingw32-gcc"
}

setup() {
	rustup target list --installed 2>/dev/null | grep -qx "$target" || rustup target add "$target"
	fetch_mingw
	use_env
	make_prefix
	say
	doctor
}

case "${1:-}" in
doctor) doctor ;;
setup) setup ;;
runner)
	# cargo runs each Windows test binary through here.
	shift
	use_env
	make_prefix
	exec "$WINE" "$@"
	;;
test)
	shift
	use_env
	make_prefix
	cargo test --target "$target" "$@"
	;;
build)
	shift
	use_env
	cargo build --target "$target" "$@"
	;;
run)
	shift
	use_env
	make_prefix
	cargo build --target "$target"
	exec "$WINE" "$root/target/$target/debug/arcanaeum.exe" "$@"
	;;
clean) rm -rf "$work" ;;
*)
	sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//' >&2
	exit 2
	;;
esac
