# 0012. Choose hair by picture, and hide the rest with a generated patch plugin

Date: 2026-11-06

## Status

Proposed

## Context

Hair packs add a head part per style to the character creation screen's one flat list, with no
search or filter: KS Hairdos adds 1,442 and Apachii SkyHair 97, on top of the game's 254. Nobody can
pick from that. RaceMenu has a search box but crashed the GOG install at startup and may do so
again; SkyUI doesn't touch the screen. The user wanted to see the hairs, click the ones to keep, and
press Apply, the way Arcanaeum already shows armours.

Ways to make the list shorter were weighed:

- **Delete the files of unwanted hairs.** The plugin still declares every hair, so each one stays in
  the list and now has no model. Invisible hair or a crash, and not undoable without the archive.
- **Edit the pack's plugin.** A copy of someone else's plugin that has to be redone for every new
  version, with every hair's record rewritten.
- **A patch plugin that overrides the unwanted hairs with the playable flag off.** The character
  creation screen lists only head parts flagged playable, so they vanish from the list. Nothing is
  deleted, and removing the patch brings every hair back.

Drawing a hair takes the model reader to a shape it didn't read: every hair in both packs is a
`BSDynamicTriShape`, whose positions are stored apart from the rest of its vertices.

## Decision

We will add a **Hair** window to the model preview (a "Hair…" button beside "From a Plugin…"): the
load order's plugins with hair on the left, and each hair as a picture on the right, drawn from its own
model and textures. Clicking a hair ticks or unticks it ("shown in game"); the window filters by name,
sex, and hairstyles or beards, and can show or hide everything listed.

**Apply** writes `Arcanaeum Hair Picks.esp`: a light plugin that overrides each unticked `HDPT` record
with its playable flag cleared, copied record for record from the plugin that wins, with FormIDs
renumbered for the patch's own masters. It adds the patch to `Plugins.txt` through skyplug-core's
`apply_changes`, at the end so it wins; ticking everything removes the patch and unticks it in
`Plugins.txt`. The patch is the only record of the picks: opening the window reads them back from it,
so there's nothing else to keep in step. Like ADR-0006, Apply refuses while Skyrim is running, but the
hair window has its own Apply, since the patch is a file written, not a switch in the list; reading
`Plugins.txt` again afterwards puts the patch in the load order shown.

The reading and writing are in skyplug-core (`hair`, which also gives it `skyplug-core`'s first plugin
writer), the model reader gains `BSDynamicTriShape`, and texture paths missing the `textures\` folder
are looked for under it, as the game does. The pictures are drawn on a worker thread of the window's
own (ADR-0002, ADR-0009), only for hairs scrolled into view, and a new request replaces the old one,
so scrolling fast doesn't draw what has gone by.

## Consequences

- The list on the character creation screen can be cut from about 1,800 to a few dozen without
  touching a pack, and cut again or undone at any time.
- A hair that isn't picked still costs memory and load time in the game: its files stay.
- The patch's picks live in a plugin, so they survive reinstalling Arcanaeum and can be seen in
  xEdit. It's tied to the load order it was made from: a plugin added after it that overrides the same
  hairs wins, and if a pack is removed the patch has a master missing.
- Hair is previewed with the model's own texture and no tint, as a head part in the game is coloured
  by the character's hair colour; a texture set (`TNAM`) that replaces the model's is not applied yet.
- This is the first thing skyplug-core writes besides `Plugins.txt`; its writer covers only
  override records of one type. Revisit if more kinds of patch are wanted: it should then be a
  general record writer, not one more special case.
