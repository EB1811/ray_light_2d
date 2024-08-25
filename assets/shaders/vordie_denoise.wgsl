
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@group(0) @binding(2)
var history_texture: texture_2d<f32>;

struct VordieLightSettings {
    u_dis_mod: f32,
    u_rays_per_pixel: i32,
    u_emission_multi: f32,
    u_max_raymarch_steps: i32,
    u_dist_mod: f32,
}
@group(0) @binding(3) var<uniform> settings: VordieLightSettings;

struct Params {
    screen_pixel_size: vec2<f32>,
    offset: f32
}
@group(0) @binding(4) var<uniform> params: Params;


struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) -> Output {
  // Very basic denoising algorithm.

  // If pixel color brighter than this, don't denoise.
  // let denoise_threshold: f32 = 0.9;
  // if (textureSample(screen_texture, texture_sampler, in.uv).x > denoise_threshold) {
  //   return textureSample(screen_texture, texture_sampler, in.uv);
  // }

  // How many 3x3s of pixels to sample.
  let denoise_count: f32 = 1.0;

  var mixed_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 1.0);

  // Sample surrounding pixels
  // for(var i: f32 = 0.0; i < denoise_count; i = i + 1.0) {
  //   for(var x: f32 = -1.0 - i; x <= 1.0 + i; x += 1.0) {
  //     for(var y: f32 = -1.0 - i; y <= 1.0 + i; y += 1.0) {
  //       let voffset: vec2<f32> = in.uv + (vec2<f32>(x, y) * 1.0 / params.screen_pixel_size);
      
  //       let pixel_color: vec4<f32> = textureSample(screen_texture, texture_sampler, voffset);

  //       mixed_color += pixel_color;
  //     }
  //   }
  //   mixed_color = mixed_color / f32(9); // 3x3
  // }

  for(var i: f32 = 0.0; i < denoise_count; i = i + 1.0) {
      for(var x: f32 = -1.0 - i; x <= 1.0 + i; x += 1.0) {
          for(var y: f32 = -1.0 - i; y <= 1.0 + i; y += 1.0) {
              // var st: vec2<f32> = in.uv;
              // st.x = st.x * inv_aspect;
              let voffset = in.uv + vec2<f32>(x, y) * 1.0 / params.screen_pixel_size;
              let pixel_color: vec4<f32> = textureSample(screen_texture, texture_sampler, voffset);

              mixed_color += pixel_color;
          }
      }
      mixed_color /= 9.0;
  }

  let col: vec3<f32> = textureSample(screen_texture, texture_sampler, in.uv).rgb;
  let integ: f32 = 2.0;
  mixed_color = vec4<f32>((1.0 - (1.0 / integ)) * mixed_color.rgb + col * (1.0 / integ), 1.0);

  // Make it less blurry by increasing the weight of the original pixel.
  mixed_color = mixed_color * 0.8 + textureSample(screen_texture, texture_sampler, in.uv) * 0.2;

  mixed_color /= f32(denoise_count);

  var out: Output;
  out.view_target = mixed_color;
  out.history = mixed_color;

  // out.view_target = textureSample(screen_texture, texture_sampler, in.uv);
  // out.history = textureSample(screen_texture, texture_sampler, in.uv);

  return out;
}