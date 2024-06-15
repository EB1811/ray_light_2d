
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

// @fragment
// fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
//     let in_diffuse   = textureSample(screen_texture, texture_sampler, in.uv);

//     return vec4<f32>(
//         in.uv.x * in_diffuse.a,
//         in.uv.y * in_diffuse.a,
//         0.0,
//         1.0
//     );

// }

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let screen_pixel_size: vec2<f32> = vec2<f32>(1024.0, 1024.0);

    var closest_dist: f32 = 9999.0;
    var closest_pos: vec2<f32> = vec2<f32>(0.0, 0.0);

    // insert jump flooding algorithm here.
    for(var x: f32 = -1.0; x <= 1.0; x += 1.0) {
        for(var y: f32 = -1.0; y <= 1.0; y += 1.0) {
            var voffset: vec2<f32> = in.uv;
            voffset += vec2<f32>(x, y) * screen_pixel_size * params.offset;

            let pos: vec2<f32> = textureSample(screen_texture, texture_sampler, voffset).xy;
            let dist: f32 = distance(pos.xy, in.uv);

            if(pos.x != 0.0 && pos.y != 0.0 && dist < closest_dist) {
                closest_dist = dist;
                closest_pos = pos;
            }
        }
    }
    
    // return vec4<f32>(closest_pos, 0.0, 1.0);

    var ret = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if(in.position.x >= params.offset && textureSample(screen_texture, texture_sampler, in.uv).r < 1.0) {
        ret = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    return ret;
}