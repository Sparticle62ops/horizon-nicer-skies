# Nicer Skies (Horizon Rust port)

Exact 1:1 Rust translation of [Nicer Skies](https://modrinth.com/mod/nicer-skies)
by ZtereoHYPE (github.com/ZtereoHYPE/nicer-skies, v1.5.2, LGPL-3.0-only).
The upstream LGPL-3.0-only license and credit are carried; see COPYING.LESSER.
Ported to Rust by sparticle.

Every upstream Java class maps 1:1 to a Rust module (see the bundle header in
`mixins/nicer_skies/runtime.rs` for the full mapping): the twinkle-star field
and its acceptance/noise rules, the cubemap skybox with the six-face paint
executor and latch, the nebula gradient stack (Shepard-interpolated keys,
altitude and time integration, domain-warped noise colors, hermite seam
smoothing), the config screen with the scale slider value mapping, and both
mixin handlers with their exact injection points.

Every Minecraft client type the port touches sits behind the host seams in
`src/seams.rs` (renderer, mesh builder, textures, executor, GUI, profiler,
noise math), a thin surface the engine binds. Log lines, field names, branch
order and quirks are kept as-is.
