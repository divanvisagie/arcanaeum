# Plan: model preview

Status: draft, 2026-11-01. Phase 0 (the spike) and phase 1 (archives, textures, resolver, in
skyplug-core) are done, see "Phase 0 results" and "Phase 1 results"; nothing else is built. Written from a session in the GOG Skyrim install
(`/tank/games/gog`), where recolouring a mod's texture (Northgirl trousers) showed the need for a
way to see a model without launching the game.

## Goal

Show an armour, clothing or other mesh as it will look with the files actually installed: the mesh,
its textures as the load order resolves them (loose files beat BSAs), on a body for scale. Used to
check a texture or mesh edit, or a newly installed mod, before a test launch.

Not a goal: matching the game's renderer (no specular, subsurface, environment maps, tint colours,
physics), animation, editing meshes, or converting LE to SE.

What it should catch: wrong colours, stretched or missing UVs, a texture path that doesn't resolve,
an LE (NIF v83) mesh that's invisible or distorted, a loose file that isn't winning.

## Phase 0 results (2026-11-01)

Spike (std-only Rust, 266 lines, plus a numpy renderer used only to check the output by eye; not kept,
since skyplug-core's `nif` module replaced it). Run against all 274 loose `.nif` files in the GOG install's `Data/meshes`.

**Gate passed: write our own reader; no need for nifly.**

- 245 files parse; the other 29 are LE (BS version 83) and are reported as such, not drawn.
  No shape has an out-of-range index, and every block parsed by the spike consumed exactly its
  declared size (NiSkinPartition checks this explicitly).
- Rendered Northgirl trousers (from `Northgirl.bsa`) and the DX Druid top/body (loose) with their
  textures; shapes, UVs and normals are right. These are the same files used for the earlier
  recolour, and the brown trousers show as brown.
- Only 3 parsed files draw nothing: trees made of the old `NiTriShape`/`NiTriShapeData` blocks, which
  SE files can still contain (65 of the files have some). They need a parser if static clutter is
  to be previewed; armour doesn't use them.

What the spike showed that the plan didn't know:

1. **Skinned SE meshes keep their geometry in `NiSkinPartition`, not in the `BSTriShape`.** The
   shape's own vertex data size is 0; vertices, UVs and normals come from the partition block
   (reached through the shape's skin instance), and the triangles from each partition's global
   "triangles copy". Nearly every armour is skinned, so this is the main path, not a corner case.
2. `BSTriShape` stores its triangle count as a 16-bit value for BS < 130 (32-bit from 130). Getting
   this wrong still "worked" on skinned meshes by accident and failed on 77 others.
3. SE vertices are full-precision float positions with half-float UVs and byte normals; computing
   the expected vertex size from the flags and comparing it with the descriptor's size caught
   every layout mistake immediately. Keep that check in the real reader.
4. The skinned geometry is already in a usable pose in model space (human height, upright, in
   Skyrim units, Z up, no skeleton needed). Phase 2 can skip skeleton posing for a first version.
5. Texture slots: `BSShaderTextureSet` gives 0 diffuse, 1 normal, 2 skin/subsurface tint, 4 env,
   5 env mask, 7 specular; paths are lower- or mixed-case, with backslashes.

Update 2026-11-06: alpha properties (test and blend) and `BSDynamicTriShape` (hair; ADR-0012) are done,
and texture paths without the leading `textures\` folder are looked for under it. What's left of the list
below is still left.

Not covered by the spike, still to do: `NiTriShape`, `BSSubIndexTriShape`'s extra data,
`BSEffectShaderProperty`, alpha properties (transparency), `NiSwitchNode`/LOD choice, per-vertex
colours, `BSDynamicTriShape` (faces/heads), BC7 textures, and BSA reading (the spike used files
extracted with the GOG repo's `bsa.py`).

Renderer note from the check: lighting that is too dark makes dark-dyed textures read as black.
Use a brighter ambient term in the real renderer.

## Phase 1 results (2026-11-01)

Decision taken: the asset code lives in **skyplug-core** (new modules `bsa`, `dds`, `assets`, in the
skyplug checkout next to this repo; uncommitted there). One new dependency, `bcdec_rs` (MIT, no_std,
pure Rust) for the BCn block decoders; `lz4_flex` was already a dependency.

- `bsa::Archive`: opens the index only, reads and decompresses files on request (LZ4 frame,
  per-file toggle, embedded names). Rejects other versions, nameless archives, corrupt counts and
  truncated data with errors.
- `dds::Dds`: BC1 to BC5 and BC7 (legacy FourCC or DX10) and uncompressed 8/24/32-bit, decoded one
  mip level at a time (`level_for(max_side)` picks the level).
- `assets::AssetResolver`: loose files first, then archives in load order, later archive wins;
  `resolve()` returns the winner and everything it shadows. `GamePaths::asset_resolver()` builds
  one from the install (base archives from `Skyrim_Default.ini` / `Skyrim.ini` / `SkyrimCustom.ini`,
  then each enabled plugin's `<name>.bsa` and `<name> - Textures.bsa`). Case-insensitive on a
  case-sensitive filesystem; paths can't escape `Data`.
- 18 new unit tests on synthetic archives and textures, plus the env-gated `real_install` sweep below; all passing, and clippy has nothing to say about the new files (it flags two older spots in `plugins.rs` and `steam.rs`).
- `cargo run --example resolve -- <game dir> --prefix <prefix> <asset path> [--out f]`.

Checked against the GOG install (read only, `SKYPLUG_TEST_GAME_DIR`, test `real_install`):

- 112 archives opened, no warnings; all 193,749 entries found by their own path; 30,477 files read and
  decompressed; 1,392 textures (BC1, BC3, BC7) decoded with no failures.
- All 28 meshes of `Northgirl.bsa` are byte-identical to what the GOG repo's Python `bsa.py` extracts.
- `textures\northgirl\northtrousers.dds` resolves to the loose recoloured file and reports that it
  hides `Northgirl.bsa`; the loose UNP `femalebody_1.dds` hides `Skyrim - Textures0.bsa`. This is the
  "why is my loose file (not) winning" answer the preview is for.

Open from this phase:

- The `<plugin> - Textures.bsa` naming rule is what this install uses and what is documented
  elsewhere; other suffixes (`- Main`, `- Meshes`) are not loaded. Check against a mod that ships one.
- `DXGI` formats beyond those listed (BC6H, float, 16-bit) return an error rather than a guess.
- Cube maps decode their first face only; texture arrays are rejected.
- Not yet published: skyplug-core's version is still 0.2.0, and Arcanaeum builds against the checkout.

## Phase 2 results (2026-11-01)

`skyplug_core::nif::parse(&bytes) -> Result<Nif>` (module `nif`, uncommitted in the skyplug checkout).
It turns the spike into a reader that returns drawable data:

- `Nif { meshes, warnings, unsupported }`. A `Mesh` has positions, normals, UVs, vertex colours,
  triangles, texture paths by slot (`nif::slot::*`), shader type, alpha (`blend`/`test`/threshold),
  `double_sided`, `hidden` and `skinned`. Node transforms are baked into every mesh, skinned
  or not (see the phase 3 correction below); skinned meshes are otherwise in the pose they're stored in.
- Block types read: `NiNode` and its common subclasses, `NiSwitchNode` (only the chosen child),
  `BSTriShape` / `BSSubIndexTriShape` / `BSMeshLODTriShape`, `NiTriShape` + `NiTriShapeData`,
  `NiSkinInstance` / `BSDismemberSkinInstance` + `NiSkinPartition`, `BSLightingShaderProperty` +
  `BSShaderTextureSet`, `BSEffectShaderProperty` (its source texture), `NiAlphaProperty`.
- LE models return a `LegacyFormat { bs_version }` error (downcast from the `anyhow::Error`). A block
  that can't be read becomes a warning and is skipped by its declared size; the file still loads.
  Block types that look like geometry but aren't handled are listed in `unsupported`.
- Hardening: every count from the file is checked against the data left before anything is allocated,
  child references and vertex attribute offsets are bounds-checked, cycles terminate, and a test
  truncates and corrupts a valid file at every byte. That test found two real panics (a child
  reference past the last block, and a descriptor with an attribute outside its vertex); both fixed.
- 10 unit tests on small NIFs built in the tests (static shape with material, skinned via partition,
  old-style `NiTriShape`, switch node, LE, bad block, bad indices, cycles, damage).

Run over the whole GOG install (`real_install_meshes`: loose files and every archive): **32,556 NIFs,
32,461 parsed, 95 LE (recognised), 0 failed; 82,632 meshes, 12,761 of them skinned.** 7,546 parsed
files have no mesh, nearly all of them (7,077) face meshes, plus particle systems, markers and
effect rigs. Only 13 files warn (37 warnings): lightning-bolt shapes whose geometry is made at runtime.

Things the whole-install run taught us (the unit tests alone had these wrong):

1. A `BSTriShape`'s data size covers the vertices **and** the triangles, not only the vertices.
2. Face meshes (`BSDynamicTriShape`) keep their positions in the shape, so their `NiSkinPartition`
   vertex stream has none; that is normal, not an error.
3. `NiTriShape` with `NiTriShapeData` is common in SE (about 65 loose files here), including
   vanilla trees. The aspen trees that came out empty in the spike now load and render.

Left for later, all reported in `unsupported` so nothing disappears silently:
`BSDynamicTriShape` (7,077 files: heads, faces), `BSLODTriShape` (40), particle systems (1,145),
`BSFurnitureMarkerNode` (314, markers, not visible). Not read at all: skeletons and bone weights,
animation, collision, tangents, a second UV set, emissive/specular/glossiness material values.

Examples: `cargo run --example nifinfo -- <file.nif> [--obj out.obj]` summarises a model and writes an
OBJ; `--example resolve` (phase 1) finds the file.

## Phase 3 results (2026-11-01)

A software renderer and a command-line front end, both in the skyplug checkout (uncommitted):

- `skyplug_core::render`: `Model::load(resolver, nif, max_texture_side)` loads each mesh's diffuse
  texture through the resolver and records, for every texture a mesh names, where it came from
  (`Found { source, hides }`, `Missing`, `Unreadable`). `render(model, camera, options)` draws it:
  orthographic camera, z-buffer, backface culling for single-sided materials, alpha test and alpha
  blend (blended meshes last, far to near), bilinear sampling, light from above-left, optional
  supersampling. Missing diffuse textures draw magenta, meshes with no texture grey. Same input,
  same pixels (a test checks it). No new dependencies in core.
- `skyplug preview <model> [--body] [--with <model>]... [--views ...] [--size N] [-o file.png]`: a
  game path or a file on disk; writes the views side by side as one PNG (uses the `png` crate, in the
  CLI crate only) and prints the texture report. About 0.3 s for a body plus an outfit.
- The orientation is Skyrim's: Z up, a character faces +Y; azimuth 0 is the front, 90 the character's right.
- Documented in the man page, the repo README and the crate README.

What looking at real renders taught us (all fixed):

1. **A skinned shape's own transform must be applied.** Phase 2 ignored it for skinned shapes, which
   looked right on body armour (identity transform) and put gloves, hands and capes at the feet:
   they are stored around their own origin and placed by the shape's translation (61 of 440 skinned
   shapes in the loose files have one). `nif::parse` now applies it to every shape.
2. **Armour files often bundle a body.** Shapes with the skin-tint shader (type 5) are bare skin; the
   game hides them under clothes. The renderer treats them, and the `--body` underlay, as an
   underlay pushed back by `Options::garment_bias` (4 units), so a garment lying slightly inside the
   body still shows. This is approximate: where the body sticks out past a garment's silhouette it
   still shows (real clothes would hide it).
3. Alpha test uses the diffuse alpha, and many mods lean on it hard: 87-92% of the DX Druid textures
   is cut out by design, which is what makes the outfit look like straps and leaves.

Checked against the GOG install (read only, test `real_install_renders`): of 1,312 sampled models,
1,064 rendered (10 blank: single flat wall/ceiling caps seen from behind, a thin rope), 248 not
drawable (LE, face meshes, no meshes); 7,421 texture references found, 55 missing (0.7%), none
unreadable. `skyplug preview` on the Northgirl trousers reports `northtrousers.dds` as the loose
recoloured file that hides `Northgirl.bsa`, which is the question this started from.

Not done, and why it's fine for now: no normal-map lighting (needs tangents), no specular/glow, no
skin or hair tint, no vertex colours, no real skinning (poses are the stored ones), no perspective
camera, no `BSDynamicTriShape` (heads), no cube-map/environment shading.

## Phase 4 results (2026-11-01)

The Arcanaeum view, `src/preview.rs` plus a few lines in `src/app.rs` (all uncommitted), recorded as
ADR-0009 (Proposed):

- **Model Preview** in the sidebar under Load Order. Models are added by game path (Enter or Add), by
  file picker (Add Files…, started in `Data/meshes`) or by dropping `.nif` files on the window; a file
  under `Data` becomes its game path. Each shows as a chip with a remove button. A **Body** checkbox
  puts the installed female body under them.
- The picture follows the window's size; drag turns it, scroll zooms, double-click or **Reset** goes
  back, and **Front / ¾ / Right / Back / Left** jump to a side. It draws a cheaper picture while the
  mouse moves and a smoother one when still.
- Beside it, every texture of every mesh: the slot, the file name, the file the game uses (loose, or
  which archive), what that hides, **MISSING** in the skin's missing colour, and a Problems list
  (models that couldn't be read, LE models, texture errors). Edit > Copy Report copies the same as text.
- **A worker thread of its own** (not the save loader's): load and render jobs, of a burst of renders
  only the newest is drawn, a render for an old load is dropped, a panic is an error in the view, and
  once a second it checks which file the game would use for each path the model needs. When one
  changes (an edited texture, or a new loose file that now wins over an archive's) the view reloads
  and redraws. Reload (Ctrl+R, F5) also starts over with the archives.
- Showing another tab (copy of the game) resets the preview.
- Screenshot mode: `ARCANAEUM_PREVIEW="meshes/a.nif;meshes/b.nif"` (and `ARCANAEUM_PREVIEW_BODY=1`) with
  `ARCANAEUM_SCREENSHOT=out.png` writes a picture of the real window; this is how the layout was checked.

Tests (added): 6 on the worker (load and report, draw, errors and LE, newest-only coalescing, change
detection, path handling) using a 645-byte NIF and a 4x4 DDS in `tests/fixtures/`, and 2 on the window
driven with real pointer events (a model drawn and its report, a drag that turns the camera, the view
buttons, Reset; a missing model reported). `cargo check --target x86_64-pc-windows-gnu` still passes.
The only failing test is the existing `search_nexus_mods_test`, which needs the network.

What looking at the real window changed: `←` isn't in the font (it drew as a box), so sources are now
indented plain text.

Not done: choosing an armour from a plugin's ARMO records (needs record parsing in skyplug-core), a
perspective camera, normal-mapped lighting, and a "worn" mode that hides the body parts the garment
covers.

## Phase 5 results (2026-11-01): the armour picker

Choosing armours from a plugin's records, in skyplug-core, the CLI and Arcanaeum (uncommitted):

- **`skyplug_core::records`**: `PluginFile::open` reads a plugin's header (flags, masters) and finds its
  top-level groups; `records(b"ARMO")` reads one group (zlib-compressed bodies, subrecords, `XXXX` sizes,
  nested groups skipped); `resolve` turns a FormID into (defining plugin, 24-bit id). A test cuts and
  corrupts a valid plugin at every byte and nothing panics. New dependency: `miniz_oxide` (zlib).
- **`skyplug_core::armour`**: `ArmourCatalog::for_game` reads every enabled plugin's `ARMA` records once;
  the last plugin to define one wins, which is how UNP Vanilla Outfits' refits show through. `armours_in(plugin)`
  reads that plugin's `ARMO` records: name (inline, or from `strings\<plugin>_english.strings` for the
  localized base plugins, found through the asset resolver), editor ID, biped slots, and the worn models
  (game paths) of its armatures: female model, else male; Argonian and Khajiit armatures dropped unless
  nothing else is left. `search` finds armours by name across all plugins.
- **CLI**: `skyplug armours` (plugins with armours), `skyplug armours <plugin> [--filter]`, and
  `skyplug preview --armour "<name or editor ID>"` (repeatable; `-` as the model draws only armours).
- **Arcanaeum**: **From a Plugin…** next to Add Files…, a window with the plugins (last loaded first, so
  mods are on top) and the chosen plugin's armours with their slots and "changes <plugin>" marks; click an
  armour to add its models, click again to remove them, **Add All Shown** for an outfit. Armours already in
  the preview are highlighted. The worker answers two new jobs (plugins, a plugin's armours) and keeps the
  catalog until Reload.

Checked against the GOG install: the catalog is built in 0.2 to 0.4 s; 65 plugins have armour records,
5,075 armour records, 94% named (the rest are skins and templates with no name record), 1,653 armatures,
none undefined. **All 5,540 model paths the catalog produces resolve to a file the game has.** UNP Vanilla
Outfits lists its four changed armours as "changes Skyrim.esm" with the UNP models; Northgirl.esp lists its
eight armours, and `skyplug preview - --armour "Northgirl Knit Jumper" ...` draws the outfit from names alone.

What the real data corrected: a mod that overrides a vanilla armour stores the name inline (it isn't
localized), so only localized plugins need the string table; and a record with a raw string ID where text
should be is shown as no name rather than as garbage. Race handling was calibrated on Skyrim.esm: 394 of its
766 armatures are DefaultRace (0x19), 99 are Argonian (0x13740), 83 Khajiit (0x13745).

Tests: 7 on the catalog (names, models, slots, the override winning, race filtering, male fallback, undefined
armatures, search, damaged plugins, string tables), 6 on the record reader, 1 worker test, 1 window test (open
the picker, choose a plugin, click an armour, click again, Add All Shown), plus `real_install_armours`.

Not done: male characters and other races (the picker offers the female, human models), ESL light-plugin
FormID compaction (resolution is by master index, which is what the files use), template armours (`TNAM`),
and listing armours that only appear as leveled-list or outfit items rather than as `ARMO` records.

## Phase 6 results (2026-11-01): drawing what the game would draw

Two things the preview drew happily that the game would not, found by use:

1. **Slot mismatch (fixed in the tools).** Moving an armour to another biped slot in its plugin (`tools/esp/reslot.py`
   in the GOG repo) left its model's skin partitions on the old slot, and the game only draws a partition whose slot
   is among its armature's, so the Northgirl strap and the Druid skirts would not have shown up; the strap
   did not, in game. The preview could not tell. Now `nif::Mesh::body_parts` reads each shape's partition slots
   (`BSDismemberSkinInstance`), `ArmourCatalog` keeps each armature's slot mask beside its model path
   (`Armour::model_slots`), and `armour::partition_problems` / `Armour::visibility_problems` say which shapes
   don't fit. Shown by `skyplug armours <plugin> --check`, as a warning by `skyplug preview --armour`, and as a red
   **SLOT MISMATCH** row in the picker (hover for the shapes). Calibrated on the GOG install: of 5,560 models,
   4 are flagged, all in vanilla (a child skin model in Skyrim.esm and one accessory shape of the Dawnguard
   vampire robes); the other 5,556 pass, so the rule has no false alarms on anything worn. Whether those 4 are
   really undrawn in game is unverified.
2. **Skeleton offsets (not done).** Pieces carry their own copy of the skeleton, and the Northgirl trousers, belt,
   strap, vest and amulet have every bone 2.54 forward and 3.29 down from the game's `skeleton_female.nif` (the
   knit jumper is 0.56 back; boots, gloves and every DX Druid and Qwib file match exactly). The game poses skinned
   meshes by the skeleton, so it moves them by the difference; the preview draws the stored pose. With the offset
   applied the trousers enclose the installed body evenly (0.0 to 1.7 units outside) where, as stored, the body
   pokes through at the back. So Northgirl pieces look out of line next to the Druid ones in the preview and
   probably not in game. Fixing it means real skinning: per-bone bind transforms (`NiSkinData`), per-vertex
   weights and bone indices from the partitions, and the game's skeleton.

Order the models are added in changes nothing (byte-identical pictures), so the "load the body first" idea is not it.

## Phase 7 results (2026-11-01): posing and texture swaps

Both are the "draw what the game would draw" work that phase 6 found was missing.

**Posing on the skeleton.** `nif::Mesh` keeps a `Skin` (bones, bind transforms from `NiSkinData`, per-vertex
weights and bone indices from the vertex stream) and `Mesh::pose(&Skeleton)` moves each vertex by the weighted
average of `bone's transform in the skeleton * bind transform`. `render::load_skeleton` reads the game's
`skeleton_female.nif`; `Model::pose` poses a model and reports what moved. Default on in `skyplug preview`
(`--no-pose` to compare) and Arcanaeum (a **Pose** checkbox; the report says how many meshes moved).

Settled with data before writing it, in a prototype against real files:
- The formula is right: for the Druid, Qwib and vanilla files, posing on a file's own bones reproduces the
  stored vertices to 0.0000. (This also showed the shape transforms applied earlier are a special case.)
- That test can't tell the two readings of the per-vertex bone index apart (any bone gives the same answer at
  a model's own bind pose). `NiSkinData` also stores per-bone (vertex, weight) lists, independent of the
  vertex stream: reading the index as **global** into the skin's bone list matches them for 48,312 of 48,317
  vertices (100%) in shapes with non-trivial partition bone lists, the partition-local reading for 21.7%.
- `NiSkinData` has the per-bone weight lists (6 bytes each) as well as the transforms.

On the GOG install, worn human models: 1,802 skinned shapes; posed on their own bones, 95% reproduce what is
stored (the rest are effects such as the monk robe's flashes and animated hand attachments); posed on the
game's skeleton, 610 move. The common moves are 3.8 units (bundled hands: the skeleton holds the fingers
differently than the files do) and 4.2 (the Northgirl pieces, whose bones are 2.54 forward and 3.29 down, as
found in phase 6). Visible effect: the Northgirl trousers and strap sit on the installed body (no body
poking through at the back) and the Druid pieces, whose skeleton matches, do not move.

Not claimed: that this is exactly what the game draws, which depends on animation; it is the skeleton's rest
pose (fingers included). Creatures hang on their own skeletons and are not posed (the female skeleton is for
humanoids).

**Texture swaps.** An armature's `MO3S` (female) / `MO2S` (male) list *(shape name, texture-set record, 3D
index)*; `ArmourCatalog` now reads `TXST` records too and gives each `Armour` its swaps per model
(`alt_textures`, paths by NIF slot with TXST's `TX00..TX07` mapped to them: diffuse, normal, environment
mask, glow, height, environment, multilayer, backlight). `armour::apply_alternate_textures` puts them on a
parsed model, matching shapes by name (the 3D index is ignored). In Arcanaeum an item is now a path plus its
swaps, so the Tartan armour and the plain one are different items though they share a mesh (chips say "alt
textures"). On the GOG install 343 armours swap textures, 1,335 swapped-in textures, 98.5% resolve; the 20 that
don't are Ulfric's clothes' known missing files. Verified by drawing the Toughened Traveler base and Tartan/Dark
variants (plain brown with grey gaiters against a dark plaid coat with laced black boots).

Tests: nif (a hand-built skinned model checked at known weights: all A, all B, half and half, own bones,
missing bones, unskinned shapes), armour (swap reading, applying, damaged swap lists), Arcanaeum worker and
window (the fixture plugin gained a variant armour with a blue texture set; the base draws red and the
variant blue; both are separate picker rows), plus `real_install_posing` and the swap check in
`real_install_armours`.

Left: animation (idle pose), male characters and other races, normal-map lighting.

## Where it lives

Two pieces, split the way ADR-0005 splits the game's files from the window:

1. **Asset loading in skyplug-core** (or a sibling crate in the skyplug workspace if core should
   stay small): BSA reader, NIF reader, DDS decoder, and an asset resolver that takes a
   `GamePaths` and the load order. Pure data in, no UI. Usable from the `skyplug` CLI too.
2. **Preview in Arcanaeum**: a view that draws what the loader returns, plus the picking UI.

A CLI entry point (`skyplug preview <nif> -o out.png`) matters as much as the window: it makes the
loader testable without a display and lets a coding agent look at a result.

## Prior art in the GOG repo (reference only, Python, not a dependency)

`/tank/games/gog/tools/esp/`: `bsa.py` (BSA v105 extract, LZ4), `nifver.py` (NIF version of a
file or folder), `deps.py` (which assets a plugin needs, loose or in a BSA, and which are missing).
They show the formats work out on real data, and give expected answers to test against.

## Phases

Each phase ends with something checkable. Stop after any of them if it isn't worth continuing.

### 0. Spike: prove NIF parsing on real files

Throwaway code. Parse the NIF header and the block types a typical SE armour uses, dump geometry
to OBJ, open it in any viewer.

Test files (real, from the GOG install; read only, found through an env var like
`ARCANAEUM_TEST_SAVES`, tests skip when it's unset):

- a static piece: Northgirl `northamulet.nif` or a vanilla amulet.
- a skinned SE armour: `meshes/Northgirl/*_1.nif` (inside `Northgirl.bsa`, so it also exercises the BSA reader).
- a skinned SE armour from loose files: DX Druid Armor under `meshes/DX/`.
- a known-bad LE mesh (v83): Relic Hunter Triss, to check the reader reports it instead of crashing.

Questions the spike answers: how many block types are needed, how skinning data must be handled to
get a sensible static pose, and whether vertex formats vary enough to make this a long tail.

**Decision gate:** if more than a handful of block types or vertex layouts fail on those files,
reconsider using an existing C++ library (nifly) through FFI instead of writing our own.

### 1. Archives, textures, resolver (in skyplug-core)

- BSA v105 (SSE) reader: folder/file records, the embedded-name flag, per-file and default
  compression (LZ4 frame). Reading only; no repacking.
- DDS decoding to RGBA: BC1, BC3, BC5, BC7 and uncompressed at least (SE textures use all of them),
  with the mip level chosen by the caller so a 4096 texture isn't decoded in full for a thumbnail.
  Check whether `image-dds` or `texpresso` fit before writing a decoder; check licences against
  BSD-3-Clause (ADR-0004).
- Resolver: given a path like `textures\northgirl\northtrousers.dds`, return the winning source:
  loose file under `Data` first, then BSAs in the order the game loads them (each plugin's
  same-named BSA in load order; the base game's archives from `Skyrim.ini`). Case-insensitive on a
  case-sensitive filesystem (the GOG install has exactly this trap with folder-name twins). Return
  *which* source won and what it shadowed, since "why is my loose file not showing" is a main use.

Check: resolve every texture of the three test meshes; results match `deps.py`.

### 2. NIF to a drawable model

- Blocks: `BSTriShape`, `BSSubIndexTriShape` (dismember), `BSLightingShaderProperty` +
  `BSShaderTextureSet` (diffuse, normal, specular paths), `NiNode` transforms, `BSSkin::Instance`
  and bone data as far as the static pose needs.
- Output: list of meshes with positions, normals, UVs, indices, and texture paths. Alpha flags.
- Unknown blocks: skip by their size where the header gives it, and report them. Never panic on a
  malformed file (ADR-0005 notes the same about saves).
- LE (v83) files: return a "legacy format" result with the version, not an attempt to draw them.
  Optionally try, labelled as probably wrong.

Check: draw order and UVs correct on the test meshes; unit tests on a few synthetic or tiny
checked-in files so CI doesn't need game data.

### 3. Rendering

Prefer a **CPU rasteriser into an RGBA image** first: headless, deterministic (so screenshots can be
tested), nothing to configure on Wayland or Windows, and a single armour is a few tens of thousands
of triangles. Turntable angles for the CLI come for free. egui shows the image as a texture, the same
way analysis.rs already shows a save's screenshot.

Move to a GPU path (egui paint callback on the existing wgpu or glow backend; check which eframe 0.35
enables here) only if orbiting is visibly slow.

Lighting: one key light plus ambient, normal map applied if cheap, alpha test. Say plainly in the UI
that it's an approximation.

Body underlay: draw the installed body for scale (`meshes/actors/character/character assets/femalebody_*.nif`
and hands/feet) with its skin texture, resolved the same way. Optional; off for items that aren't worn.

### 4. Arcanaeum view

First version, no record parsing: **Preview** in the sidebar with "Open model…" (file picker over the
game's `Data`) and a text field for a game-relative path. Orbit with drag, zoom with scroll, reset.
Beside the picture: the textures it resolved, from where (loose / which BSA), and warnings (missing,
legacy format, shadowed).

Second version: **pick from a plugin.** Needs reading ARMO/ARMA records in skyplug-core: EDID, FULL,
male/female world model paths, additional races. Then Load Order gets a "Preview" action on a
plugin that lists its armours and opens one. This is the part that makes it more than a file viewer.

Third: **reload on change.** Watch the shown files and redraw when one changes, so a recolour appears
as the file is saved. Needs a file watcher (e.g. `notify`) or a simple mtime poll.

### 5. Tests and docs

- Unit tests per reader with small fixtures.
- Real-data tests behind an env var, read only, skipped when unset, like the saves tests. Never write to the game folder.
- A headless Arcanaeum test driving the view through `App::draw`, like `src/app/tests.rs`.
- README section, ADR(s) (see below), and a `make` target for the CLI render if it's in this repo.

## Decisions to record as ADRs (not yet written; none is signed off)

1. Where the asset code lives (skyplug-core vs a new crate vs Arcanaeum itself).
2. CPU rasteriser first, GPU later (or the reverse).
3. Own NIF reader vs FFI to nifly (depends on the phase 0 gate).

## Risks

- **NIF long tail.** Mods ship odd vertex layouts, extra data, and older blocks. Mitigation: the
  phase 0 gate and a reader that reports what it skipped.
- **Skinning.** A skinned mesh isn't in its worn shape until the skeleton pose is applied. A bind
  pose or the body's own coordinates may be good enough; unknown until phase 0.
- **Case-insensitive paths** on Linux, and Windows path separators inside NIFs and BSAs.
- **Scope creep** into a general Skyrim asset browser. Keep to "does this mod look right".
- **Dependency size and build time** from texture codecs; check before adding.
- skyplug-core is shared with the `skyplug` CLI and currently only does plugins, saves and Steam
  lookup; adding formats widens it. A separate crate in the same workspace may be cleaner.

## Open questions

- Is a 3D viewport something Patina should offer as a component, or is this app-local?
- Should the preview read the game's load order from `Plugins.txt` (what is enabled) or from the
  filesystem alone? The first is correct; the second works with no plugins set up.
- Windows: BSA and NIF code is portable; check nothing in the file lookup assumes `/`.
- GOG and Steam differ in exe version, not in these formats, so one reader should serve both tabs.
- Do we want the LE-to-SE conversion later? Out of scope here, but a clear "this mesh is LE" is the
  first step toward it.
