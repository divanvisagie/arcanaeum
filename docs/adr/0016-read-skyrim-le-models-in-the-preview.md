# 0016. Read Skyrim LE models in the preview

Date: 2026-11-14

## Status

Proposed

## Context

skyplug-core's NIF reader read only Skyrim SE's format (stream version 100) and refused Skyrim LE's (83) with
a `LegacyFormat` error, "so callers can say 'this is an LE mesh' instead of drawing it wrong". In practice the
game loads many LE models fine, and the preview left them out: a character's Triss boots (an LE model, worn and
drawn in the game) were missing from the preview of an outfit, along with other pieces; after this change the same
outfit draws 54 meshes instead of 44. A preview that shows an outfit with pieces missing misleads more than one
that draws an LE piece slightly differently.

## Decision

We will read LE models. The two formats lay out every block the reader uses the same way except that a skinned
shape keeps its vertices in the SE skin partition but in `NiTriShapeData` in LE, where the skin partition holds only
each vertex's weights and bone indices (local to its partition; the reader maps them to the skin instance's bone
list, as the SE buffer already has them). Other stream versions are still refused with `LegacyFormat`.

## Consequences

- Outfits with LE pieces draw whole. The pieces are posed on the skeleton like the rest.
- The preview no longer says which pieces are LE; a model that is LE and also draws wrongly in the game would now be
  drawn (and maybe wrongly) instead of refused. The install notes' habit of checking `nifver.py` first still holds.
- Only what the reader already covered is read from LE files: one partition per shape's weights, four weights per
  vertex, the block types listed in `nif.rs`. Anything else is a warning for that block, as for SE.
