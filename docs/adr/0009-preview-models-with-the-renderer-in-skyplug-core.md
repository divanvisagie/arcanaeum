# 0009. Preview models with the renderer in skyplug-core, on a worker of their own

Date: 2026-11-01

## Status

Proposed

## Context

Installing and editing mods (an armour, a recoloured texture) meant a test launch of the game to see
whether the result looked right and whether a loose file was beating an archive. The user wanted to
see a model, with the textures the game would use, inside Arcanaeum, and considered it a feature of
the app rather than a separate tool. Arcanaeum already knows the game folder and load order, and
reads the game's files through skyplug-core (ADR-0005).

Where the pieces live was weighed: the file formats (archives, textures, models), the renderer, and
the window. A renderer inside Arcanaeum would have left `skyplug` (which has the same game lookup)
without a way to render for scripts and for checking changes from a terminal. A GPU renderer in egui
(wgpu paint callbacks) would be faster for orbiting but needs a display for every test and every
automated check, and nothing it showed could be compared byte for byte. The renderer's job is to check
shape, texture mapping, colour and which file wins, not to match the game's shading.

Orbiting asks for a new picture on every frame, so drawing can't happen on the UI thread (the rule
behind ADR-0002) and can't queue one picture per frame behind a slow one.

## Decision

We will keep reading archives, textures and models, resolving which file wins, and drawing in
skyplug-core (`bsa`, `dds`, `nif`, `assets`, `render`), a CPU renderer with no GPU or window. Arcanaeum
draws what it returns: a "Model Preview" entry in the sidebar, with models added by path, file picker
or drop, a drag-to-turn picture, and a list of every texture with the file the game uses and what that
file hides. `skyplug preview` is the same thing for the command line.

The preview gets a worker thread of its own (`preview.rs`), separate from the save loader's so a slow
save scan doesn't delay a picture and the other way round. Its jobs are load and render; of a burst of
render jobs only the newest is drawn, and a render for a load that is no longer current is dropped.
Each job runs under `catch_unwind`, so a model that panics the reader is an error in the view. The
worker also looks once a second at which file the game would use for each path the model needs, and
tells the view when one changes (an edited texture, or a new loose file that now wins), which reloads.

Armours are chosen from the plugins' own records rather than by file path. skyplug-core reads the
`ARMO` and `ARMA` records of every enabled plugin (`records`, `armour`), takes the last plugin to
change an armature as the one that counts, and names the models the load order really wears; the
worker keeps that catalog and answers the picker's two questions (which plugins have armours, and
what are one plugin's). Names of the localized base plugins come from their string tables.

## Consequences

- A recoloured texture shows up as it is saved, and "is my loose file being used?" is answered by the
  report without launching the game.
- The same code serves the command line and the tests; the pictures are deterministic, so they can be
  checked without a display.
- Orbiting is as fast as the CPU can draw one picture (tens of milliseconds for an outfit); it draws a
  cheaper picture while dragging and a smoother one when still. If that proves too slow for big
  scenes, a GPU path can replace the drawing without changing the loading.
- It is a preview, not the game's renderer: no normal, specular or glow maps, no skin or hair tint, no
  real skinning, no face meshes. It says so rather than pretending.
- Arcanaeum's build depends on more of skyplug-core, which grows with the formats it reads; models it
  can't read show as errors and warnings, not crashes.
- The picker shows what the load order really wears, so a refit (UNP Vanilla Outfits) shows its models
  for vanilla armours. It picks female models for human races; male models, and other races' armatures,
  aren't offered, and an armour that lists no armature of its own shows "no models".
- Two threads read the game's files now. The preview's worker reads the archives' indexes once and keeps
  them until Reload, so it doesn't see a newly added archive until then (a changed loose file is seen
  at once).
