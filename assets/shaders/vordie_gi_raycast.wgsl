
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
var history_texture: texture_2d<f32>;

@group(0) @binding(5)
var<uniform> time: f32;

const PI: f32 = 3.141596;

fn random(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn dist_tonemap(col: vec3<f32>, dist: f32) -> vec3<f32> {
    let screen_pixel_size: vec2<f32> = vec2<f32>(textureDimensions(u_scene_data, 0).xy);
    return col * (1.0 - dist / min(screen_pixel_size.x, screen_pixel_size.y));
}

struct SurfaceResult {
    emissive: f32,
    colour: vec3<f32>,
}
fn get_surface(uv: vec2<f32>, ray_origin: vec2<f32>) -> SurfaceResult {
    let screen_pixel_size: vec2<f32> = vec2<f32>(textureDimensions(u_scene_data, 0).xy);
    let emissive_data = textureSample(u_scene_data, texture_sampler, uv);

    let color_by_dist = dist_tonemap(emissive_data.rgb, distance(ray_origin*screen_pixel_size, uv*screen_pixel_size));
    
    return SurfaceResult(
      max(color_by_dist.r, max(color_by_dist.g, color_by_dist.b)) * settings.u_emission_multi,
      color_by_dist.rgb
    );
}

struct RaymarchResult {
    hit: bool,
    hit_pos: vec2<f32>,
    random_pixel_pos: vec2<f32>,
}
fn raymarch(origin: vec2<f32>, dir: vec2<f32>, time: f32, reso: vec2<f32>) -> RaymarchResult {
    var current_dist: f32 = 0.0;
    for (var i: i32 = 0; i < settings.u_max_raymarch_steps; i = i + 1) {
        var sample_point: vec2<f32> = origin + dir * current_dist;
        
        // early exit if we hit the edge of the screen.
        if (sample_point.x > 1.0 || sample_point.x < 0.0 || sample_point.y > 1.0 || sample_point.y < 0.0) {
            return RaymarchResult(
                false,
                vec2<f32>(0.0),
                vec2<f32>(0.0),
            );
        }

        // var dist_to_surface: f32 = textureLoad(u_distance_data, vec2<i32>(i32(sample_point.x), i32(sample_point.y)), 0).r / settings.u_dist_mod;
        var dist_to_surface: f32 = textureSample(u_distance_data, texture_sampler, sample_point).r / settings.u_dist_mod;

        // we've hit a surface if distance field returns 0 or close to 0 (due to our distance field using a 16-bit float
        // the precision isn't enough to just check against 0).
        if (dist_to_surface < 0.5 / max(reso.x, reso.y)) {
            // Get a random light pixel along our marched ray for GI.
            // let rand = random(((f32(i) + 1.0) / 5) * vec2<f32>(time, -time));
            // let random_pixel_pos: vec2<f32> = origin + rand * (sample_point - origin);

            return RaymarchResult(
                true,
                sample_point,
                // random_pixel_pos,
                vec2<f32>(0.0),
            );
        }



        // if we don't hit a surface, continue marching along the ray.
        current_dist = current_dist + dist_to_surface;
    }

    return RaymarchResult(
        false,
        vec2<f32>(0.0),
        vec2<f32>(0.0),
    );
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
    let reso: vec2<f32> = vec2<f32>(textureDimensions(u_scene_data, 0).xy);

    var pixel_emis: f32 = 0.0;
    var pixel_col: vec3<f32> = vec3<f32>(0.0);
    var rand_pixel_col: vec3<f32> = vec3<f32>(0.0);

    let rand2pi: f32 = random(in.uv * vec2<f32>(time, -time)) * 2.0 * PI;
    let golden_angle: f32 = PI * 0.7639320225; // Magic number for good ray distribution.

    var hit_col: vec3<f32> = vec3<f32>(0.0);

    // Cast our rays.
    for(var i: i32 = 0; i < settings.u_rays_per_pixel; i = i + 1) {
        // Get our ray dir by taking the random angle and adding golden_angle * ray number.
        let cur_angle: f32 = rand2pi + golden_angle * f32(i);
        let ray_dir: vec2<f32> = normalize(vec2<f32>(cos(cur_angle), sin(cur_angle)));
        let ray_origin: vec2<f32> = in.uv;

        var ray_res: RaymarchResult = raymarch(ray_origin, ray_dir, time, reso);
        if(ray_res.hit) {
            let pixel_surface: SurfaceResult = get_surface(ray_res.hit_pos, ray_origin);

            // Also collect the random pixel for GI.
            // rand_pixel_col += textureSample(history_texture, texture_sampler, ray_res.random_pixel_pos).rgb;

            pixel_emis += pixel_surface.emissive;
            pixel_col += pixel_surface.colour;
        }
    }

    var col = (pixel_col / pixel_emis) * (pixel_emis / f32(settings.u_rays_per_pixel));

    // Add the GI using a little bit of the random pixel color.
    // rand_pixel_col /= f32(settings.u_rays_per_pixel);
    // col += rand_pixel_col * 0.1;



    // Color correction and filters.
    // TODO: Make this a parameter, or another shader pass.
    // col = col * 0.8;
    // col = col * (1.0 / (1.0 + col * 0.5));

    return vec4<f32>(lin_to_srgb(vec4<f32>(col, 1.0)), 1.0);
}