# 0007. Find a game outside Steam through a game folder and Wine prefix set in Settings

Date: 2026-10-24

## Status

Accepted

## Context

The user keeps a GOG copy of Skyrim Special Edition, run under umu with GE-Proton in its own
Wine prefix, outside any Steam library. File > Set Game Folder couldn't open it on Linux:
skyplug-core only found the user's files in the Steam library's compatdata prefix, and refused a
game folder that wasn't under `steamapps/common`. GOG's build also names its per-user folders
`Skyrim Special Edition GOG` (in `AppData/Local` and `My Games`), so even the right prefix gave
the wrong `Plugins.txt` and saves.

Detecting the prefix (from umu's or Lutris's config, say) was considered and ruled out for now:
every launcher keeps it somewhere else, and a copy run by a script, as here, records it nowhere.
Asking for it is one folder pick, done once.

## Decision

We will take the Wine prefix as a setting beside the game folder, both kept in `config.toml`
(`game_folder`, `wine_prefix`) and set in a Settings window (File > Settings…, Ctrl+,) that
replaces the File menu's Set Game Folder and Find Game Through Steam.

Settings is the same form for every copy: plain folder fields with Browse…, saved together. Steam
isn't a special case in it. An early version showed "Steam's install" for an empty game folder and
"Use Steam's" beside the prefix, so Steam's copy was set up differently from any other, which the
user found confusing. Instead, Fill In From Steam writes Steam's copy's folders into the fields,
and on a first run the first copy is filled in that way. An empty game folder is just unset.

skyplug-core 0.2.0 does the work: `GameInstall::new(game_dir, prefix)` takes the prefix, and
`Edition` tells GOG's build from Steam's by the `goggame-*.info` files its installer leaves.
On Windows the prefix row is hidden, since the files are in the user's own folders.

## Consequences

- A GOG copy under Wine or umu works on Linux, as does a Steam copy run in another prefix.
- The game folder and prefix are separate fields; a user who sets only the folder of a copy
  outside Steam gets skyplug-core's error asking for the prefix, in the load order view.
- Steam's copy is stored as plain folders, so if Steam moves it to another library, the user
  fills it in from Steam again.
- Arcanaeum builds against `../skyplug` until skyplug-core 0.2.0 is on crates.io, as it did
  for 0.1.1 (ADR-0005).
- Epic's and the Microsoft Store's builds name their folders differently again and aren't
  recognised; adding them belongs in skyplug-core's `Edition`.
