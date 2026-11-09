// The screen copy drawn back over the whole picture: one triangle covering it, the texture at the fragment's place.
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct BlitOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_blit(@builtin(vertex_index) index: u32) -> BlitOut {
    var out: BlitOut;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    out.clip = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);
    return out;
}

@fragment
fn fs_blit(in: BlitOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
