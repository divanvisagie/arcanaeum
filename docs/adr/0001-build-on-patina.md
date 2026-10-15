# 0001. Rebuild Arcanaeum's window on Patina

Date: 2026-10-15

## Status

Accepted

## Context

Arcanaeum was already an egui app, but with its own ad hoc UI: a left panel holding a light/dark
switch, a "Fix resolution" button that opened a placeholder window, and two side-by-side lists
(characters, then saves) painted with hard-coded greys and black text that broke in dark mode.
Plugin status used hard-coded green and red. It sat on eframe 0.36 with the system title bar.

The user asked for it to be rewritten to Patina's UI rules, so it behaves like the rest of the
family (Photograph, UnAmp, Notebook, Calendar).

## Decision

We will build Arcanaeum's window on [Patina](https://github.com/Sector-F-Labs/patina)
(`patina-ui`):

- Patina's own window frame. Word menus on the left of the title bar (File, Edit, View, Skins)
  and the app's output action on the right (ADR-0003), with View > System Title Bar as the
  escape hatch.
- The desktop's light/dark setting followed through `patina::appearance`, replacing the old
  in-app theme switch.
- Skins in the suite format through `patina::skin`, from Patina's built-ins and
  `~/.config/arcanaeum/skins/`. No home skin: it starts in Default. Plugin status colours are
  the app's own skin keys under `[app.arcanaeum]` (`installed`, `available`, `missing`,
  `included`), each falling back to the active visuals.
- One sidebar of characters, each a collapsible list of its saves (newest first), instead of two
  side-by-side lists; the chosen save in the central panel; a status bar holding both bottom
  corners.
- Settings move from an empty `~/.config/Arcaneum/config.json` to
  `~/.config/arcanaeum/config.toml` (save folder, skin, system title bar).

## Consequences

- Arcanaeum looks and behaves like the rest of the Patina family and takes the same skins.
- egui went back from 0.36 to 0.35, Patina's version; it moves to a new egui only together with
  Patina.
- The "Fix resolution" button is gone. It only ever opened a placeholder, and
  `fix_resolution()` writes a hard-coded 3440×1440 into the real `SkyrimPrefs.ini`, so it needs
  a real design before it comes back as a menu item.
- The old config folder (`Arcaneum`, misspelt) is no longer read or created; it never held
  anything.
