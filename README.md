# Arcanaeum

Arcanaeum will analyse your Skyrim save games to determine which mods you used for that save.

Pick a character and one of their saves, and Arcanaeum lists every plugin the save was made
with: base game, Creation Club, installed (found in the game's `Data` folder), or not installed,
with download links from the Vortex masterlist where it has them, beside the screenshot the game
took when saving. **Copy Report** puts the list on the clipboard. Each column of the plugin list
has a filter in its header: the load order index, any part of the name, and which statuses to
show.

It's a [Patina](https://github.com/Sector-F-Labs/patina) app: an egui desktop app with its own
window frame, the desktop's light/dark setting, and the suite's skins.

### Future Plans

- Automatic download and application of mods
- Automatic fixing of common problems like resolution management for ultrawide monitors

![Arcanaeum in the Default skin](docs/screenshot-default.png)

## Shortcuts

| Keys | Action |
|------|--------|
| Ctrl+O | Open a save folder |
| Ctrl+R, F5 | Reload the folder and the open save |
| Ctrl+Shift+C | Copy the open save's report |
| Ctrl+Q | Quit |

## Skins

Skins come from Patina's built-ins and `~/.config/arcanaeum/skins/`. Arcanaeum's own keys colour
a plugin's status, and each falls back to the skin's text, warning, error and weak colours:

```toml
[app.arcanaeum]
installed = "#4caf50"
available = "#e0a030"
missing = "#d04040"
included = "#808080"
```

## Build

Run `make` to list the targets:

- `make run`: run the app.
- `make test`: run the tests.
- `make check`: tests and clippy.
- `make screenshot`: regenerate `docs/screenshot-*.png` from Divan's newest save, in a scratch
  config (`CHARACTER=Name` for another character).
- `make build`: a release binary.

Settings are kept in `~/.config/arcanaeum/config.toml`. Design decisions are in
[`docs/adr/`](docs/adr/README.md).

## Resources

## Info about the file format

https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format
https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions

### Info about matching plugins

https://www.nexusmods.com/news/13921

### List of skyrim mods can be found here

https://github.com/loot/skyrimse

## Origin of the name

https://en.uesp.net/wiki/Skyrim:The_Arcanaeum
