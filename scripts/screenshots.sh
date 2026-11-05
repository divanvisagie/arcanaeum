#!/usr/bin/env bash
# Regenerate docs/screenshot-*.png from a real character's newest save, in a scratch config so the
# settings of a running copy are never touched. `make screenshot` calls this.
#
#   CHARACTER   whose newest save to show (Divan)
#   SKIN        the skin (Everforest Dark)
#   PICKER      the plugin the armour picker shows (Dragonborn.esm), and PICKER_FILTER for it (Nordic)
#   STEAM_GAME_FOLDER, STEAM_WINE_PREFIX, GOG_GAME_FOLDER, GOG_WINE_PREFIX
#               a tab for each copy in the title bar. Give all four, or none: with none the one
#               tab is Steam's copy, found by itself, as on a first run.
#
# Put machine-specific values in local.mk (ignored by git).
set -euo pipefail
cd "$(dirname "$0")/.."

character=${CHARACTER:-Divan}
skin=${SKIN:-Everforest Dark}
picker=${PICKER:-Dragonborn.esm}
picker_filter=${PICKER_FILTER:-Nordic}

scratch=$PWD/target/scratch
rm -rf "$scratch"
mkdir -p "$scratch/config/arcanaeum" "$scratch/data" docs
[ ! -f "$HOME/.config/user-dirs.dirs" ] || cp "$HOME/.config/user-dirs.dirs" "$scratch/config/"

if [ -n "${STEAM_GAME_FOLDER:-}${STEAM_WINE_PREFIX:-}${GOG_GAME_FOLDER:-}${GOG_WINE_PREFIX:-}" ]; then
	for v in STEAM_GAME_FOLDER STEAM_WINE_PREFIX GOG_GAME_FOLDER GOG_WINE_PREFIX; do
		[ -n "${!v:-}" ] || { echo "screenshots: $v is not set (give all four install variables, or none)" >&2; exit 1; }
	done
	cat >"$scratch/config/arcanaeum/config.toml" <<TOML
shown = 0

[[installs]]
name = "Steam"
game_folder = "$STEAM_GAME_FOLDER"
wine_prefix = "$STEAM_WINE_PREFIX"

[[installs]]
name = "GOG"
game_folder = "$GOG_GAME_FOLDER"
wine_prefix = "$GOG_WINE_PREFIX"
TOML
fi

cargo build -q
shot() {
	local out=$1
	shift
	echo "docs/$out"
	env XDG_CONFIG_HOME="$scratch/config" XDG_DATA_HOME="$scratch/data" \
		ARCANAEUM_CHARACTER="$character" ARCANAEUM_SKIN="$skin" ARCANAEUM_SCREENSHOT="docs/$out" "$@" \
		cargo run -q 2>/dev/null
}

shot screenshot-save.png
shot screenshot-load-order.png ARCANAEUM_LOAD_ORDER=1
shot screenshot-outfit.png ARCANAEUM_OUTFIT=1
shot screenshot-picker.png ARCANAEUM_PICKER="$picker" ARCANAEUM_PICKER_FILTER="$picker_filter"
