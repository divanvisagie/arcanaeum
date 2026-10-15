# 0005. Read and write the game's files through skyplug-core

Date: 2026-10-15

## Status

Accepted

## Context

Arcanaeum had its own `.ess` parser, Steam library lookup and `Data` scan. The user's other
tool, [skyplug](https://github.com/divanvisagie/skyplug), had grown better versions of all
three, plus what Arcanaeum lacked: reading `Plugins.txt` and `Skyrim.ccc` so it knows whether
a plugin actually loads, switching plugins on and off with a backup, an approximate load
order, and saves sorted by when they were written. skyplug had already split these into a
library, `skyplug-core`, so other tools wouldn't re-implement the file formats.

Arcanaeum's own code also answered some things wrongly: any plugin in `Data` counted as
"installed" even if `Plugins.txt` had it switched off; Creation Club meant "the name starts
with cc"; saves were ordered by save number, which several saves can share; and the parser
panicked on a truncated file.

skyplug-core only knew the Proton (Linux) locations of the user's files, while Arcanaeum
also ran on Windows. The user wanted all functionality and Windows support kept, and the
Windows support added to skyplug-core rather than kept in Arcanaeum.

## Decision

We will depend on `skyplug-core` for finding the game, parsing saves, and reading and
writing `Plugins.txt`, and delete Arcanaeum's own versions (`save_file_parser`, `utils`,
`mod_search::data_scanner`). Arcanaeum keeps what only it has: the Vortex masterlist's
download links, the Patina window, and the old `features/` code, now pointed at
skyplug-core's paths.

skyplug-core 0.1.1 gains Windows support without an API change: `GameInstall`'s per-user
paths resolve into the Proton prefix on Linux and into the user's own `AppData\Local` and
Documents (through the `dirs` crate, so a Documents folder moved to OneDrive is found) on
Windows. Until 0.1.1 is published, Arcanaeum builds against the skyplug checkout next to it
(`path = "../skyplug/crates/skyplug-core"` alongside `version = "0.1.1"`).

## Consequences

- Statuses are right: Active and Disabled come from `Plugins.txt`, Creation Club from
  `Skyrim.ccc`, and a save's newest is the one written last.
- One parser and one Steam lookup for both tools; a fix in one helps the other.
- Arcanaeum's build needs `../skyplug` until skyplug-core 0.1.1 is on crates.io; then the
  `path` can go.
- skyplug-core still only finds Steam on Windows at its default location, and only Skyrim
  Special Edition. File > Set Game Folder covers a copy installed elsewhere.
- Like Arcanaeum's old parser, skyplug-core doesn't read zlib-compressed saves, or the
  light (ESL) plugin list SE saves keep after the regular one.
