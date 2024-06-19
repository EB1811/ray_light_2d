
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct VordieLightSettings {
    setting: f32
}
@group(0) @binding(2) var<uniform> settings: VordieLightSettings;

struct Params {
    offset: f32
}
@group(0) @binding(3) var<uniform> params: Params;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let in_diffuse   = textureSample(screen_texture, texture_sampler, in.uv);

    return vec4<f32>(
        in.uv.x * in_diffuse.a,
        in.uv.y * in_diffuse.a,
        0.0,
        1.0
    );

}