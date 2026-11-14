# 0015. Draw the model preview on the GPU, the way the game shades it

Date: 2026-11-14

## Status

Proposed

## Context

The model preview ([ADR-0009](0009-preview-models-with-the-renderer-in-skyplug-core.md)) is a CPU
rasteriser in skyplug-core: one diffuse map, one light. Making it show a material's shine
([ADR-0014](0014-show-the-shine-in-the-model-preview.md)) fixed one thing, but the aim is to see a model as
the game draws it (normal maps, specular maps, proper lighting) so mods can be tuned without launching the
game. Done per pixel on the CPU that is much heavier than what ADR-0009 measured, and the CPU path is already
slow where it is used a lot: the hair picker draws one CPU picture per hair, one after another.

ADR-0009 kept the renderer on the CPU so tests and `skyplug preview` need no display and results are
deterministic. That is still true of the CPU renderer, but a second, GPU renderer for the window doesn't have
to share its limits. Arcanaeum already runs on eframe's wgpu backend (egui-wgpu 0.35, wgpu 29), so no new
graphics dependency is needed.

## Decision

We will draw the Model Preview's picture with a wgpu renderer of Arcanaeum's own (`src/gpu.rs`,
`src/gpu_model.wgsl`) and make wgpu the window's renderer explicitly. The worker flattens the loaded model
into a `Scene` (vertex and index data plus the textures; skyplug-core's new `Model::parts` and
`Model::load_shaded` supply them, normal maps included). The picture is an egui paint callback: it renders into
an offscreen multisampled target and blits it into egui's pass, so nothing is read back to the CPU, and
dragging or zooming redraws at the frame rate.

Shading, first version: diffuse (alpha test or blend as the mesh says), tangent-space normal map (tangent
frame from derivatives, so meshes need no tangents), a key and a fill light plus ambient, a Blinn-Phong
highlight scaled by the normal map's alpha, in linear light. Still to do: the environment map, skin and hair
tints, glow, shadows, and checking the normal-map conventions (`BUMP_Y`) against the game.

The same pipelines run offscreen (`gpu::Offscreen`) for the ignored `gpu_shot` test, which renders models from a
real install to a PNG without a window. skyplug-core stays CPU-only and keeps `skyplug preview`.

The hair picker's pictures (112 px thumbnails and the 448 px one) are drawn the same way, by the hair worker on a
device of its own (`HairGpu`), with transparent backgrounds. Measured on 200 of the 1788 hairs here, in a dev build
(the `hair_thumbs_on_the_gpu_against_the_cpu` ignored test): the GPU draw alone was no faster than the CPU's
(about 3 ms against 2 ms a thumbnail); what cost time was finding, decompressing and decoding textures, about 12 to
18 ms a hair, and 200 hairs name 423 textures of which 26 are different. So the worker keeps decoded textures
between hairs (`skyplug_core::render::TextureCache`) and the renderer keeps uploaded ones (`keeping_textures`):
200 hairs went from about 22 ms each to about 2.6 ms each. The worker's CPU render job in the preview is unchanged.

## Consequences

- The preview can show what normal maps and specular maps do to a model, and orbiting is live. On the dev
  machine, rendering and reading back a 520 px picture takes about 32 ms in a debug build; the window path has no
  readback.
- It needs a GPU (or a software Vulkan such as llvmpipe); without one eframe's wgpu start-up fails. The CPU
  renderer remains for the CLI and tests, so there are now two renderers whose shading will drift unless the
  CLI picks up the same shading later.
- The first picture of a model pays for uploading it (about 1.2 s for a hood, body and textures in a debug
  build). The mip levels are generated on the CPU, which may be a good part of that; it wasn't timed separately.
- It is a close approximation of vanilla-style shading, not the game with Community Shaders or other shader
  mods; the aim is to see problems, not to match pixels.
