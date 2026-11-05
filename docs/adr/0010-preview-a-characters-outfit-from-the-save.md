# 0010. Preview a character's outfit by reading what the save says is worn

Date: 2026-11-05

## Status

Proposed

## Context

The model preview (ADR-0009) draws armours picked by hand. The user wanted to open a save and see its
character wearing their outfit in the same view. The save knows what is worn, but Arcanaeum read only its
header, screenshot and plugin list; the outfit is in the body, in the change form of the player
reference (`0x14`): the inventory, where each item has a list of extra data and a worn item carries
`Worn` or `WornLeft`.

A spike read a real SE save (120,000 change forms) and found the inventory and the worn items, which
turned out to be a coherent outfit on distinct slots. The format is documented only on UESP, partly as
unknowns, and the leading fields of the player's form (initial data, havok data, package data) were not
decoded. Options weighed: decoding the whole form from its flags (complete, but each field is a chance
to be wrong in a format that is mostly "unknown"), and finding the inventory by looking for the block
that parses cleanly from its first byte to its last (needs only the inventory and extra data layouts).
Where it lives was weighed as for ADR-0009: the parser in Arcanaeum would leave `skyplug` unable to do it.

## Decision

We will read the worn items in skyplug-core (`outfit`: `outfit`, `read_outfit_from_file`), and map them
to armours with `ArmourCatalog::armours_for`, so `skyplug outfit` and `skyplug preview --outfit` do the
same as the app. The inventory is found by scanning the player's form for the largest block that parses
from end to end; a block with an extra data type the reader doesn't know is not taken for one.

Arcanaeum's save view gets "Preview Outfit" (Special Edition saves only). It sends the preview worker a
job that reads the save and the armours, and what comes back replaces the preview's models, with the body
on, as if the pieces had been picked one after another. A line of notices under the heading says what
isn't drawn. A worn model the renderer can't read is skipped with a warning, not fatal.

## Consequences

- A save's outfit is one click, with the same textures, poses and report as any other model.
- The character isn't drawn, only the outfit on the installed body: no face, hair or skin tint, which
  the save holds as race and character-creation data, not armour. Male characters were first shown on the
  female body with a note; [ADR-0011](0011-draw-either-body-and-let-a-save-choose-it.md) draws them on a male one.
- Items made in game (enchanted or renamed copies, FormIDs `FF…`) have no record in a plugin and are
  counted, not shown. Worn weapons and torches carry `Worn` too; they aren't armour and are counted.
- The scan is a heuristic. An extra data type nobody has seen makes the read fail with "couldn't find the
  player's inventory" rather than show a wrong outfit; decoding the form's leading fields from its flags
  would remove the scan. Offsets in the save count from the file as if the two size fields before a
  compressed body weren't there, found by experiment on one game version.
- Only SE saves; LE saves have a different layout.
- Models in the LE format (some old mods) aren't drawn, so such a piece is missing from the picture.
