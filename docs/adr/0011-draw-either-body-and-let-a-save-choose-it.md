# 0011. Draw either body, and let a save's character choose it

Date: 2026-11-05

## Status

Proposed

## Context

The model preview (ADR-0009) drew only the female side of the game: each armour's female mesh, the
installed female body and the female skeleton. ADR-0010 drew a save's outfit that way too, so a male
character was shown on a female body with a note saying so, which looked wrong in the README and was
wrong for the model the save's character really wears. The game has a male mesh for most armours
(`MOD2`, beside the female `MOD3`), a male body, hands and feet, and a male skeleton.

The choice of body had to work in two situations: someone picking armours by hand, who should be able
to look at either, and someone opening a save, where the character already is one or the other and
a switch would show something that character never wears. Options for carrying both bodies through
the preview: keep a catalog setting that picks the models when records are read (a switch would then
have to read every picked armour again, and the items added so far no longer say which armour they
came from), or keep both sets on each armour and choose at the point of drawing.

## Decision

We will keep both model sets on each armour in skyplug-core (`Armour::female`, `Armour::male`,
`Armour::models_for`), with `BodySex` naming the body and choosing its body files and skeleton
(`BodySex::body`, `BodySex::skeleton`). An armature with a model for only one sex gives it to both, as
it did for female before, rather than leaving an armour empty. `skyplug armours` and `skyplug preview`
take `--sex`, and `preview --outfit` uses the save's character unless told otherwise.

In Arcanaeum, what is added to the preview is a `Piece`: a model file, or an armour with its models for
each body. A **Female | Male** choice in the toolbar picks which are drawn, so switching reads nothing
again beyond the new load. An outfit opened from a save sets the body to the character's and locks the
choice, with a hover that says why; removing the last piece of the outfit unlocks it. An Argonian or a
Khajiit is drawn with the human models and body, with a note, since the preview has no beast bodies.

## Consequences

- A male character's outfit is drawn on a male body with the male meshes, and the README no longer has
  to apologise for it.
- `Armour`'s model fields moved into the two sets: a breaking change for anything using
  `Armour::models`, `model_slots` or `alt_textures`, and `visibility_problems` takes the sex.
- The picker's slot checks are for one body, so switching the body reads the plugin's armours again.
- Typed paths and picked files are the same on either body, since a file is one model; choosing a male
  body doesn't turn a `/f/` mesh into its male equivalent.
- The lock is a rule about the outfit, not a record of the character: add armours by hand to a locked
  outfit and they follow the locked body; clear it to choose again.
- Only humanoid bodies: Argonian and Khajiit bodies, and the child races, aren't drawn.
- This changes what ADR-0009 says about male models not being offered and what ADR-0010 says about male
  characters.
