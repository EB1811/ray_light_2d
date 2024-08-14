
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct VordieLightSettings {
    u_dis_mod: f32,
    u_rays_per_pixel: i32,
    u_emission_multi: f32,
    u_max_raymarch_steps: i32,
    u_dist_mod: f32,
}
@group(0) @binding(2) var<uniform> settings: VordieLightSettings;

struct Params {
    offset: f32
}
@group(0) @binding(3) var<uniform> params: Params;


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // TODO: Make this a input.
    let screen_pixel_size: vec2<f32> = vec2<f32>(800.0, 800.0);

    var closest_dist: f32 = 9999999.9;
    var closest_pos: vec2<f32> = vec2<f32>(0.0, 0.0);

    let uv: vec2<f32> = in.uv;

    for(var x: f32 = -1.0; x <= 1.0; x += 1.0) {
        for(var y: f32 = -1.0; y <= 1.0; y += 1.0) {
            let voffset: vec2<f32> = uv + (vec2<f32>(x, y) * params.offset / screen_pixel_size);

            let pos: vec2<f32> = textureSample(screen_texture, texture_sampler, voffset).xy;
            let dist: f32 = length(pos - uv);

            if(pos.x != 0.0 && pos.y != 0.0 && dist < closest_dist) {
                closest_dist = dist;
                closest_pos = pos;
            }
        }
    }
    
    return vec4<f32>(closest_pos, 0.0, 1.0);
}