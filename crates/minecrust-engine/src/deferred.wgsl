struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen triangle trick
    // vi = 0 -> x = -1, y = 1,  uv = (0, 0)
    // vi = 1 -> x = -1, y = -3, uv = (0, 2)
    // vi = 2 -> x = 3,  y = 1,  uv = (2, 0)
    let x = f32(i32(vi & 2u) * 2 - 1);
    let y = f32(1 - i32(vi & 1u) * 4);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Bind Group 0: Fallback Deferred inputs
@group(0) @binding(0) var t_albedo: texture_2d<f32>;
@group(0) @binding(1) var t_normal: texture_2d<f32>;
@group(0) @binding(2) var t_mrao: texture_2d<f32>;
@group(0) @binding(3) var t_depth: texture_depth_2d;
@group(0) @binding(4) var s_sampler: sampler;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    world_time: f32,
    frame_index: u32,
    enable_rt: u32,
    _padding: u32,
};
@group(0) @binding(5)
var<uniform> camera: CameraUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = textureSample(t_depth, s_sampler, in.uv);
    let albedo_tex = textureSample(t_albedo, s_sampler, in.uv);
    let normal_tex = textureSample(t_normal, s_sampler, in.uv);

    // world_time is in 0 to 24000.
    // 6000 is noon, 18000 is midnight.
    let angle = (camera.world_time / 24000.0) * 2.0 * 3.14159265;
    let sun_dir = normalize(vec3<f32>(cos(angle), sin(angle), 0.3));

    // Dynamic sky colors based on world_time
    let sky_y = sin(angle);
    let day_sky_top = vec3<f32>(0.2, 0.45, 0.85);
    let day_sky_bot = vec3<f32>(0.55, 0.75, 0.95);
    let sunset_sky = vec3<f32>(0.9, 0.35, 0.15);
    let night_sky_top = vec3<f32>(0.005, 0.008, 0.02);
    let night_sky_bot = vec3<f32>(0.015, 0.02, 0.04);

    // If background (depth is 1.0)
    if (depth >= 1.0) {
        var sky_top = night_sky_top;
        var sky_bot = night_sky_bot;

        if (sky_y > 0.0) {
            let sunset_factor = smoothstep(0.0, 0.15, sky_y) * (1.0 - smoothstep(0.1, 0.45, sky_y));
            let day_factor = smoothstep(0.35, 0.55, sky_y);
            sky_top = mix(night_sky_top, day_sky_top, day_factor);
            sky_bot = mix(night_sky_bot, day_sky_bot, day_factor);

            sky_top = mix(sky_top, sunset_sky, sunset_factor);
            sky_bot = mix(sky_bot, sunset_sky * 1.1, sunset_factor);
        } else {
            // Moon phase sky glow slightly
            let moon_factor = smoothstep(0.0, 0.2, -sky_y);
            sky_top = mix(night_sky_top, night_sky_top * 1.2, moon_factor);
            sky_bot = mix(night_sky_bot, night_sky_bot * 1.5, moon_factor);
        }

        let final_sky = mix(sky_bot, sky_top, in.uv.y);
        return vec4<f32>(final_sky, 1.0);
    }

    // Geometry lighting
    let normal = normalize(normal_tex.xyz);
    let albedo = albedo_tex.rgb;

    // Define sunlight and ambient colors
    var light_color = vec3<f32>(0.0);
    var ambient = vec3<f32>(0.05); // Minimal baseline ambient

    if (sky_y > 0.0) {
        // Sun is up
        let sun_intensity = smoothstep(0.0, 0.2, sky_y);
        light_color = vec3<f32>(1.0, 0.95, 0.85) * sun_intensity;
        ambient = mix(vec3<f32>(0.05), vec3<f32>(0.2, 0.23, 0.28), sun_intensity);
    } else {
        // Moon light (cool blueish glow)
        let moon_intensity = smoothstep(0.0, 0.2, -sky_y);
        light_color = vec3<f32>(0.15, 0.25, 0.45) * moon_intensity * 0.4;
        ambient = mix(vec3<f32>(0.05), vec3<f32>(0.04, 0.05, 0.07), moon_intensity);
    }

    let light_dir = select(-sun_dir, sun_dir, sky_y > 0.0);
    let NdotL = max(dot(normal, light_dir), 0.0);
    let lit_color = albedo * (ambient + light_color * NdotL);

    return vec4<f32>(lit_color, 1.0);
}
