
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // TODO: White = emiter, black = occluder

    let in_diffuse   = textureSample(screen_texture, texture_sampler, in.uv);

    return vec4<f32>(
        in_diffuse.r,
        in_diffuse.g,
        in_diffuse.b,
        in_diffuse.a
    );

}