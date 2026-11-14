# 0014. Show a material's shine in the model preview

Date: 2026-11-14

## Status

Proposed

## Context

Editing a mod's meshes (a fur hood converted to SE and given fur layers, for the first case) meant a
launch of the game for every look, because the preview ([ADR-0009](0009-preview-models-with-the-renderer-in-skyplug-core.md))
drew only the diffuse texture under one light. The first fur layers came out washed-out and glossy in
the game and looked fine in the preview: they copied a shader with the specular flag, glossiness 1.0 and
white specular, and had no normal map to scale it. The game takes how much shine each point gets from the
alpha of the normal map, and a material with no map gets all of it.

Two ways to see that without the game were weighed. A GPU renderer in egui (wgpu paint callbacks;
Arcanaeum already runs on eframe's wgpu backend) would orbit smoothly, but ADR-0009 kept the renderer on
the CPU so tests and `skyplug preview` need no display, and a second renderer would have to be kept in
step with the first. Speed wasn't the problem: a release build draws four views of a hood, loading
included, in about 0.4 s (0.2 s for one view), and `cargo run` was slower only because the dev profile
leaves `skyplug-core` unoptimised (2.05 s for the same). What was missing was the shading, which is
wanted in either renderer.

## Decision

We will read the shine of a `BSLightingShaderProperty` that has the specular flag (glossiness, specular
colour and strength) in skyplug-core's NIF reader (`Mesh::specular`), load the normal map of such a mesh
for its alpha, and add a Blinn-Phong highlight to the CPU renderer, strength scaled by that alpha, or by
full where the material has no normal map, as the game does. Normal-mapped lighting, glow, environment
maps and tints are still not drawn. `skyplug preview` and Arcanaeum's Model Preview get it together.
Arcanaeum's dev profile optimises `skyplug-core` (`[profile.dev.package.skyplug-core]`).

## Consequences

- A glossy material, or a layer that is far shinier than its neighbours, shows as such in a picture
  made from a terminal, without the game, and the tests can check it.
- It approximates the classic specular model, not the game's renderer with whatever shader mods it
  runs: the highlight's shape and strength are a guide, not a match, and the light is the preview's one
  light, not the scene's.
- Normal maps still don't change the lighting, so bumps and a normal map's own effect on the highlight
  are missing.
- A GPU view remains possible later (ADR-0009 left that open); it would need the same shading.
