
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var u_distance_data: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct VordieLightSettings {
    u_rays_per_pixel: i32,
    u_emission_multi: f32,
    u_max_raymarch_steps: i32,
    u_dist_mod: f32,
}
@group(0) @binding(2) 
var<uniform> settings: VordieLightSettings;

@group(0) @binding(3)
var u_scene_data: texture_2d<f32>;

@group(0) @binding(4)
var<uniform> time: f32;

const PI: f32 = 3.141596;

fn random(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn get_surface(uv: vec2<f32>, emissive: ptr<function, f32>, colour: ptr<function, vec3<f32>>) {
    let emissive_data = textureSample(u_scene_data, texture_sampler, uv);
    *emissive = max(emissive_data.r, max(emissive_data.g, emissive_data.b)) * settings.u_emission_multi;
    *colour = emissive_data.rgb;
}

fn raymarch(origin: vec2<f32>, dir: vec2<f32>, hit_pos: ptr<function, vec2<f32>>) -> bool {
    var current_dist: f32 = 0.0;
    for (var i: i32 = 0; i < settings.u_max_raymarch_steps; i = i + 1) {
        var sample_point: vec2<f32> = origin + dir * current_dist;
        
        // early exit if we hit the edge of the screen.
        if (sample_point.x > 1.0 || sample_point.x < 0.0 || sample_point.y > 1.0 || sample_point.y < 0.0) {
            return false;
        }

        // var dist_to_surface: f32 = textureLoad(u_distance_data, vec2<i32>(i32(sample_point.x), i32(sample_point.y)), 0).r / settings.u_dist_mod;
        var dist_to_surface: f32 = textureSample(u_distance_data, texture_sampler, sample_point).r / settings.u_dist_mod;

        // we've hit a surface if distance field returns 0 or close to 0 (due to our distance field using a 16-bit float
        // the precision isn't enough to just check against 0).
        if (dist_to_surface < 0.001) {
            *hit_pos = sample_point;
            return true;
        }

        // if we don't hit a surface, continue marching along the ray.
        current_dist = current_dist + dist_to_surface;
    }
    return false;
}

fn lin_to_srgb(color: vec4<f32>) -> vec3<f32> {
    let x: vec3<f32> = color.rgb * 12.92;
    let y: vec3<f32> = 1.055 * pow(clamp(color.rgb, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0)), vec3<f32>(0.4166667, 0.4166667, 0.4166667)) - 0.055;
    var clr: vec3<f32> = color.rgb;
    clr.r = select(y.r, x.r, color.r < 0.0031308);
    clr.g = select(y.g, x.g, color.g < 0.0031308);
    clr.b = select(y.b, x.b, color.b < 0.0031308);
    return clr;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var pixel_emis: f32 = 0.0;
    var pixel_col: vec3<f32> = vec3<f32>(0.0);

    let rand2pi: f32 = random(in.uv * vec2<f32>(time, -time)) * 2.0 * PI;
    let golden_angle: f32 = PI * 0.7639320225; // Magic number for good ray distribution.

    var hit_col: vec3<f32> = vec3<f32>(0.0);

    // Cast our rays.
    for(var i: i32 = 0; i < settings.u_rays_per_pixel; i = i + 1) {
        // Get our ray dir by taking the random angle and adding golden_angle * ray number.
        let cur_angle: f32 = rand2pi + golden_angle * f32(i);
        let ray_dir: vec2<f32> = normalize(vec2<f32>(cos(cur_angle), sin(cur_angle)));
        let ray_origin: vec2<f32> = in.uv;

        var hit_pos: vec2<f32>;
        var hit: bool = raymarch(ray_origin, ray_dir, &hit_pos);
        if(hit) {
            var mat_emissive: f32;
            var mat_colour: vec3<f32>;
            get_surface(hit_pos, &mat_emissive, &mat_colour);

            pixel_emis += mat_emissive;
            pixel_col += mat_colour;
        }
    }

    pixel_col /= pixel_emis;
    pixel_emis /= f32(settings.u_rays_per_pixel);

    var col = pixel_emis * pixel_col;

    // Color correction and filters.
    // TODO: Make this a parameter, or another shader pass.
    col = col * 0.7;
    col = col * (1.0 / (1.0 + col * 0.5));

    return vec4<f32>(lin_to_srgb(vec4<f32>(col, 1.0)), 1.0);
    // return vec4<f32>(col, 1.0);
}