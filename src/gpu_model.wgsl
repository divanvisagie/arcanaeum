// The model preview's shader: one pass per mesh, drawn straight into a multisampled target.
//
// Colour is worked in linear light: diffuse maps are sampled as sRGB, lit, then encoded back.
// Per fragment: the diffuse map (alpha-tested or blended as the mesh's NiAlphaProperty says), a
// tangent-space normal map when the mesh has one (the tangent frame comes from screen-space
// derivatives, so meshes need no tangents), a key light from above-left of the camera, a fill light
// from the other side and a sky/ground ambient, and a Blinn-Phong highlight for default-shader
// materials with the specular flag, scaled by the normal map's alpha as the game's specular map.

struct Globals {
    center: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    toward: vec4<f32>,
    // x, y: clip units per game unit across and up; z: clip depth per game unit.
    scale: vec4<f32>,
    key: vec4<f32>,
    fill: vec4<f32>,
};

struct Part {
    // rgb: specular colour, a: strength.
    spec: vec4<f32>,
    // x: glossiness, y: alpha-test threshold 0..255 (negative: none), z: depth push (game units), w: flags.
    misc: vec4<f32>,
};

const HAS_NORMAL_MAP: u32 = 1u;
const HAS_SPECULAR: u32 = 2u;
const BROKEN: u32 = 4u;
const NO_DIFFUSE: u32 = 8u;
const BLEND: u32 = 16u;

const KEY_COLOUR: vec3<f32> = vec3<f32>(1.0, 0.88, 0.72);
const FILL_COLOUR: vec3<f32> = vec3<f32>(0.62, 0.72, 0.95);
const SKY_AMBIENT: vec3<f32> = vec3<f32>(0.40, 0.45, 0.55);
const GROUND_AMBIENT: vec3<f32> = vec3<f32>(0.22, 0.20, 0.18);

// Skyrim's normal maps are DirectX style; flip if bumps look sunken under the key light.
const BUMP_Y: f32 = 1.0;

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var<uniform> part: Part;
@group(1) @binding(1) var diffuse_tex: texture_2d<f32>;
@group(1) @binding(2) var normal_tex: texture_2d<f32>;
@group(1) @binding(3) var samp: sampler;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs(v: VIn) -> VOut {
    let q = v.pos - g.center.xyz;
    var o: VOut;
    // Orthographic, like the CPU preview: nearer is smaller z. An underlay (the body) is pushed back.
    let z = 0.5 - dot(q, g.toward.xyz) * g.scale.z + part.misc.z * g.scale.z;
    o.clip = vec4<f32>(dot(q, g.right.xyz) * g.scale.x, dot(q, g.up.xyz) * g.scale.y, z, 1.0);
    o.world = v.pos;
    o.nrm = v.nrm;
    o.uv = v.uv;
    return o;
}

fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let flags = u32(part.misc.w);
    var base = textureSample(diffuse_tex, samp, i.uv);
    if (flags & BROKEN) != 0u {
        base = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    } else if (flags & NO_DIFFUSE) != 0u {
        base = vec4<f32>(0.445, 0.445, 0.445, 1.0);
    }
    if part.misc.y >= 0.0 && base.a * 255.0 <= part.misc.y {
        discard;
    }

    var n = normalize(i.nrm);
    if !front {
        n = -n;
    }
    var spec_mask = 1.0;
    if (flags & HAS_NORMAL_MAP) != 0u {
        let t = textureSample(normal_tex, samp, i.uv);
        spec_mask = t.a;
        // Tangent frame from derivatives of position and UV (Schuler, "Normal Mapping Without Precomputed Tangents").
        let dp1 = dpdx(i.world);
        let dp2 = dpdy(i.world);
        let duv1 = dpdx(i.uv);
        let duv2 = dpdy(i.uv);
        let dp2perp = cross(dp2, n);
        let dp1perp = cross(n, dp1);
        let tan = dp2perp * duv1.x + dp1perp * duv2.x;
        let bit = dp2perp * duv1.y + dp1perp * duv2.y;
        let inv = inverseSqrt(max(max(dot(tan, tan), dot(bit, bit)), 1e-20));
        let tn = normalize(vec3<f32>(t.r * 2.0 - 1.0, (t.g * 2.0 - 1.0) * BUMP_Y, t.b * 2.0 - 1.0));
        n = normalize(tan * inv * tn.x + bit * inv * tn.y + n * tn.z);
    }

    // Light colours as an overcast-to-sunny day gives them: a warm key, a cool fill, a sky-tinted ambient.
    // The game multiplies every light term, specular included, by the light's colour, so a material with a
    // strongly coloured specular (a blue (0.08, 0.08, 1.0) is not rare) is tamed by warm light.
    let key = max(dot(n, g.key.xyz), 0.0);
    let fill = max(dot(n, g.fill.xyz), 0.0);
    let ambient = mix(GROUND_AMBIENT, SKY_AMBIENT, 0.5 + 0.5 * n.z);
    var colour = base.rgb * (ambient + 0.80 * key * KEY_COLOUR + 0.22 * fill * FILL_COLOUR);

    if (flags & HAS_SPECULAR) != 0u {
        // Vanilla: HdotN ^ shininess, shininess as the file has it (no clamp), times the specular colour
        // and strength, the normal map's alpha, and the key light's colour.
        let h = normalize(g.key.xyz + g.toward.xyz);
        let lobe = pow(max(dot(n, h), 1e-4), max(part.misc.x, 0.0));
        colour = colour + lobe * part.spec.a * spec_mask * part.spec.rgb * KEY_COLOUR * step(0.0, dot(n, g.key.xyz));
    }

    var alpha = 1.0;
    if (flags & BLEND) != 0u {
        alpha = base.a;
    }
    return vec4<f32>(srgb_encode(colour), alpha);
}

// ---------------------------------------------------------------- blit into egui's pass

@group(0) @binding(0) var blit_tex: texture_2d<f32>;
@group(0) @binding(1) var blit_samp: sampler;
@group(0) @binding(2) var<uniform> blit_u: vec4<f32>; // x: 1 when egui's target is an sRGB format

struct BOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn blit_vs(@builtin(vertex_index) k: u32) -> BOut {
    // One triangle covering the viewport.
    let p = vec2<f32>(f32((k << 1u) & 2u), f32(k & 2u));
    var o: BOut;
    o.clip = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn blit_fs(i: BOut) -> @location(0) vec4<f32> {
    // The target holds colour already multiplied by coverage (an MSAA resolve of straight colours
    // over a transparent clear), which is what egui's premultiplied blending wants.
    var c = textureSample(blit_tex, blit_samp, i.uv);
    if blit_u.x > 0.5 {
        c = vec4<f32>(srgb_decode(c.rgb), c.a);
    }
    return c;
}
