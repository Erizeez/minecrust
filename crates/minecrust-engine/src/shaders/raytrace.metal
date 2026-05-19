#include <metal_stdlib>
#include <metal_raytracing>

using namespace metal;
using namespace raytracing;

struct CameraUniform {
    float4x4 view_proj;
    float4x4 inv_view_proj;
    float4x4 prev_view_proj;
    float4 camera_pos;
    float world_time;
    uint frame_index;
    uint enable_rt;
    uint _padding;
};

// PRNG: Hash function for random number generation
uint hash(uint seed) {
    seed ^= seed >> 16;
    seed *= 0x7feb352dU;
    seed ^= seed >> 15;
    seed *= 0x846ca68bU;
    seed ^= seed >> 16;
    return seed;
}

float random_float(thread uint& seed) {
    seed = hash(seed);
    return float(seed) / 4294967295.0;
}

float3 random_in_unit_sphere(thread uint& seed) {
    float theta = random_float(seed) * 2.0 * 3.14159265;
    float phi = acos(2.0 * random_float(seed) - 1.0);
    float r = pow(random_float(seed), 1.0/3.0);
    return float3(r * sin(phi) * cos(theta), r * sin(phi) * sin(theta), r * cos(phi));
}

kernel void rt_main(
    uint2 tid [[thread_position_in_grid]],
    texture2d<float, access::write> output_texture [[texture(0)]],
    texture2d<float, access::read> albedo_tex [[texture(1)]],
    texture2d<float, access::read> normal_tex [[texture(2)]],
    texture2d<float, access::read> mrao_tex [[texture(3)]],
    texture2d<float, access::read> depth_tex [[texture(4)]],
    texture2d<float, access::sample> history_tex [[texture(5)]],
    constant CameraUniform& camera [[buffer(6)]],
    instance_acceleration_structure tlas [[buffer(7)]]
) {
    if (tid.x >= output_texture.get_width() || tid.y >= output_texture.get_height()) {
        return;
    }

    uint seed = hash(tid.x + tid.y * output_texture.get_width() + camera.frame_index * 1337);

    float world_time = camera.world_time;
    float time_angle = (world_time / 24000.0) * 2.0 * 3.14159265;
    
    // Sun and moon direction
    float3 sun_dir = normalize(float3(cos(time_angle), sin(time_angle), 0.3));
    float is_day = sun_dir.y > 0.0 ? 1.0 : 0.0;
    
    float3 main_light_dir = is_day ? sun_dir : normalize(float3(-cos(time_angle), -sin(time_angle), -0.3));
    float light_intensity = max(main_light_dir.y, 0.1); // softer at horizon
    if (!is_day) light_intensity *= 0.2; // moon is weaker

    // Sky color interpolation
    float3 day_sky = float3(0.5, 0.7, 1.0);
    float3 sunset_sky = float3(0.8, 0.4, 0.2);
    float3 night_sky = float3(0.05, 0.05, 0.1);
    
    float3 sky_color;
    if (sun_dir.y > 0.1) {
        sky_color = mix(sunset_sky, day_sky, smoothstep(0.1, 0.3, sun_dir.y));
    } else if (sun_dir.y > -0.1) {
        sky_color = mix(night_sky, sunset_sky, smoothstep(-0.1, 0.1, sun_dir.y));
    } else {
        sky_color = night_sky;
    }

    float2 current_uv = (float2(tid) + 0.5) / float2(output_texture.get_width(), output_texture.get_height());
    float2 ndc_xy = current_uv * 2.0 - 1.0;
    ndc_xy.y = -ndc_xy.y;
    
    float depth = depth_tex.read(tid).r;
    float4 clip_pos = float4(ndc_xy, depth, 1.0);
    float4 world_pos_h = camera.inv_view_proj * clip_pos;
    float3 world_pos = world_pos_h.xyz / world_pos_h.w;

    float4 albedo = albedo_tex.read(tid);
    
    if (albedo.a < 0.1) {
        // Sky reprojection
        float4 prev_clip = camera.prev_view_proj * float4(world_pos_h.xyz, 0.0); // Infinite distance approximation for sky
        float2 prev_ndc = prev_clip.xy / prev_clip.w;
        float2 prev_uv = prev_ndc * 0.5 + 0.5;
        prev_uv.y = 1.0 - prev_uv.y;
        
        float3 current_color = sky_color;
        
        float blend_weight = 0.1;
        if (prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0 || camera.frame_index < 2) {
            blend_weight = 1.0;
        }
        
        constexpr sampler linear_sampler(coord::normalized, filter::linear, address::clamp_to_edge);
        float4 history = history_tex.sample(linear_sampler, prev_uv);
        
        float3 final_color = mix(history.rgb, current_color, blend_weight);
        output_texture.write(float4(final_color, 1.0), tid);
        return;
    }

    float4 normal = normal_tex.read(tid);
    float4 mrao = mrao_tex.read(tid);

    float metallic = mrao.r;
    float roughness = mrao.g;
    float ao = mrao.b;

    if (length(mrao.rgb) < 0.01) {
        metallic = 0.0;
        roughness = 1.0;
        ao = 1.0;
    }

    // Reprojection
    float4 prev_clip = camera.prev_view_proj * float4(world_pos, 1.0);
    float2 prev_ndc = prev_clip.xy / prev_clip.w;
    float2 prev_uv = prev_ndc * 0.5 + 0.5;
    prev_uv.y = 1.0 - prev_uv.y;

    // Prevent float precision drift blur when stationary
    if (length(prev_uv - current_uv) < 1e-4) {
        prev_uv = current_uv;
    }

    float3 final_color;

    if (camera.enable_rt == 0) {
        // Vanilla Minecraft lighting fallback
        float face_lighting = 1.0;
        if (abs(normal.x) > 0.5) {
            face_lighting = 0.6;
        } else if (abs(normal.z) > 0.5) {
            face_lighting = 0.8;
        } else if (normal.y < -0.5) {
            face_lighting = 0.5; // Bottom face
        }
        
        final_color = albedo.rgb * face_lighting;
        
        // Simple distance fog
        float dist = length(world_pos - camera.camera_pos.xyz);
        float fog_factor = clamp((dist - 30.0) / 30.0, 0.0, 1.0);
        final_color = mix(final_color, sky_color, fog_factor);
    } else {
        // RT is ON
        float3 ray_origin = world_pos + normal.xyz * 0.01;
        float3 view_dir = normalize(camera.camera_pos.xyz - world_pos);
        
        float3 jittered_light_dir = normalize(main_light_dir + random_in_unit_sphere(seed) * 0.05);

        ray shadow_ray;
        shadow_ray.origin = ray_origin;
        shadow_ray.direction = jittered_light_dir;
        shadow_ray.min_distance = 0.0;
        shadow_ray.max_distance = 1000.0;

        intersector<instancing, triangle_data> i;
        i.assume_geometry_type(geometry_type::triangle);
        
        intersection_result<instancing, triangle_data> shadow_res = i.intersect(shadow_ray, tlas);

        float n_dot_l = max(dot(normal.xyz, jittered_light_dir), 0.0);
        
        float ambient_intensity = is_day ? 0.4 : 0.1;
        float3 ambient = albedo.rgb * sky_color * ambient_intensity * ao;
        float3 diffuse = albedo.rgb * n_dot_l * light_intensity;
        
        if (!is_day) {
            diffuse *= float3(0.6, 0.8, 1.0);
        } else {
            diffuse *= mix(float3(1.0, 0.8, 0.6), float3(1.0), smoothstep(0.0, 0.3, sun_dir.y));
        }

        if (shadow_res.type != intersection_type::none) {
            diffuse *= 0.1;
        }

        final_color = ambient + diffuse;

        if (roughness < 0.8 || metallic > 0.1) {
            float3 ideal_reflect_dir = reflect(-view_dir, normal.xyz);
            float3 reflect_dir = normalize(ideal_reflect_dir + random_in_unit_sphere(seed) * roughness * 0.5);
            
            if (dot(reflect_dir, normal.xyz) > 0.0) {
                ray reflect_ray;
                reflect_ray.origin = ray_origin;
                reflect_ray.direction = reflect_dir;
                reflect_ray.min_distance = 0.0;
                reflect_ray.max_distance = 100.0;
                
                intersection_result<instancing, triangle_data> reflect_res = i.intersect(reflect_ray, tlas);
                
                float3 reflection_color = sky_color;
                if (reflect_res.type != intersection_type::none) {
                    reflection_color = float3(0.2, 0.2, 0.2); 
                }
                
                float3 f0 = mix(float3(0.04), albedo.rgb, metallic);
                float n_dot_v = max(dot(normal.xyz, view_dir), 0.0);
                float3 fresnel = f0 + (1.0 - f0) * pow(1.0 - n_dot_v, 5.0);
                
                final_color = mix(final_color, reflection_color, fresnel);
            }
        }
    }

    // Depth-based History Rejection
    float expected_prev_linear_depth = prev_clip.w;
    
    // Use nearest sampler! Since we lack sub-pixel camera jitter, linear sampling causes severe recursive blurring/smearing during movement.
    constexpr sampler history_sampler(coord::normalized, filter::nearest, address::clamp_to_edge);
    float4 history = history_tex.sample(history_sampler, prev_uv);
    float actual_prev_linear_depth = history.a;
    
    float blend_weight = 0.1;
    
    // Bypass TAA completely if RT is off
    if (camera.enable_rt == 0) {
        blend_weight = 1.0;
    }
    
    // Strict depth threshold: max 0.25m to perfectly separate 1m blocks
    float depth_threshold = clamp(0.02 * expected_prev_linear_depth, 0.05, 0.25);
    
    if (abs(expected_prev_linear_depth - actual_prev_linear_depth) > depth_threshold) {
        blend_weight = 1.0;
    }
    
    if (prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0 || camera.frame_index < 2) {
        blend_weight = 1.0;
    }
    
    float3 accumulated_color = mix(history.rgb, final_color, blend_weight);
    
    // Store current linear depth in alpha channel for next frame
    float current_linear_depth = (camera.view_proj * float4(world_pos, 1.0)).w;
    output_texture.write(float4(accumulated_color, current_linear_depth), tid);
}
