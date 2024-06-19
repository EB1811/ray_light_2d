
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
    let screen_pixel_size: vec2<f32> = vec2<f32>(1024.0, 1024.0);

    var closest_dist: f32 = 9999.0;
    var closest_pos: vec2<f32> = vec2<f32>(0.0, 0.0);

    // insert jump flooding algorithm here.
    for(var x: f32 = -1.0; x <= 1.0; x += 1.0) {
        for(var y: f32 = -1.0; y <= 1.0; y += 1.0) {
            let voffset: vec2<f32> = in.uv + (vec2<f32>(x, y) * params.offset) / screen_pixel_size;

            let pos: vec2<f32> = textureSample(screen_texture, texture_sampler, voffset).xy;
            let dist: f32 = distance(pos.xy, in.uv.xy);

            if(pos.x != 0.0 && pos.y != 0.0 && dist < closest_dist) {
                closest_dist = dist;
                closest_pos = pos;
            }
        }
    }
    
    return vec4<f32>(closest_pos, 0.0, 1.0);
}