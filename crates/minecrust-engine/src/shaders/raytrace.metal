#include <metal_stdlib>
#include <metal_raytracing>

using namespace metal;
using namespace raytracing;

struct CameraUniform {
    float4x4 view_proj;
    float4x4 inv_view_proj;
    float4 camera_pos;
    float world_time;
    float3 _padding;
};

kernel void rt_main(
    uint2 tid [[thread_position_in_grid]],
    texture2d<float, access::write> output_texture [[texture(0)]],
    texture2d<float, access::read> albedo_tex [[texture(1)]],
    texture2d<float, access::read> normal_tex [[texture(2)]],
    texture2d<float, access::read> mrao_tex [[texture(3)]],
    texture2d<float, access::read> depth_tex [[texture(4)]],
    constant CameraUniform& camera [[buffer(5)]],
    instance_acceleration_structure tlas [[buffer(6)]]
) {
    if (tid.x >= output_texture.get_width() || tid.y >= output_texture.get_height()) {
        return;
    }

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

    float4 albedo = albedo_tex.read(tid);
    
    if (albedo.a < 0.1) {
        output_texture.write(float4(sky_color, 1.0), tid);
        return;
    }

    float depth = depth_tex.read(tid).r;
    float4 normal = normal_tex.read(tid);

    float2 uv = float2(tid) / float2(output_texture.get_width(), output_texture.get_height());
    float2 ndc_xy = uv * 2.0 - 1.0;
    ndc_xy.y = -ndc_xy.y;
    float4 clip_pos = float4(ndc_xy, depth, 1.0);
    float4 world_pos_h = camera.inv_view_proj * clip_pos;
    float3 world_pos = world_pos_h.xyz / world_pos_h.w;

    float3 ray_origin = world_pos + normal.xyz * 0.01;
    
    ray r;
    r.origin = ray_origin;
    r.direction = main_light_dir;
    r.min_distance = 0.0;
    r.max_distance = 1000.0;

    intersector<instancing, triangle_data> i;
    i.assume_geometry_type(geometry_type::triangle);
    
    intersection_result<instancing, triangle_data> result = i.intersect(r, tlas);

    // Basic lighting
    float n_dot_l = max(dot(normal.xyz, main_light_dir), 0.0);
    
    float ambient_intensity = is_day ? 0.4 : 0.1;
    float3 ambient = albedo.rgb * sky_color * ambient_intensity;
    float3 diffuse = albedo.rgb * n_dot_l * light_intensity;
    
    if (!is_day) {
        diffuse *= float3(0.6, 0.8, 1.0); // Moon light tint
    } else {
        diffuse *= mix(float3(1.0, 0.8, 0.6), float3(1.0), smoothstep(0.0, 0.3, sun_dir.y)); // Sunrise tint
    }

    if (result.type != intersection_type::none) {
        diffuse *= 0.1;
    }

    float3 final_color = ambient + diffuse;
    output_texture.write(float4(final_color, 1.0), tid);
}
