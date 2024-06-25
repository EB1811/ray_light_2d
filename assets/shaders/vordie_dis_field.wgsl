
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct VordieLightSettings {
    u_dis_mod: f32
}
@group(0) @binding(2) var<uniform> settings: VordieLightSettings;


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let in_diffuse = textureSample(screen_texture, texture_sampler, in.uv);

    let dist: f32 = distance(in_diffuse.xy, in.uv);
    let mapped: f32 = clamp(dist * settings.u_dis_mod, 0.0, 1.0);
    
    return vec4<f32>(vec3<f32>(mapped), 1.0);
}