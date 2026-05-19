#include <metal_stdlib>
#include <metal_raytracing>

using namespace metal;
using namespace raytracing;

struct CameraUniform {
    float4x4 view_proj;
    float4x4 inv_view_proj;
    float4 camera_pos;
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

    float4 albedo = albedo_tex.read(tid);
    
    // Simple verification: if albedo is empty, return sky color
    if (albedo.a < 0.1) {
        output_texture.write(float4(100.0/255.0, 149.0/255.0, 237.0/255.0, 1.0), tid);
        return;
    }

    float depth = depth_tex.read(tid).r;
    float4 normal = normal_tex.read(tid);
    // float4 mrao = mrao_tex.read(tid); // Metallic, Roughness, AO, Emissive?

    // Unproject depth to world position
    float2 uv = float2(tid) / float2(output_texture.get_width(), output_texture.get_height());
    // WGPU NDC is x: -1 to 1, y: 1 to -1, z: 0 to 1
    float2 ndc_xy = uv * 2.0 - 1.0;
    ndc_xy.y = -ndc_xy.y;
    float4 clip_pos = float4(ndc_xy, depth, 1.0);
    float4 world_pos_h = camera.inv_view_proj * clip_pos;
    float3 world_pos = world_pos_h.xyz / world_pos_h.w;

    // Hardcoded sun direction (e.g. from above and slightly angled)
    float3 sun_dir = normalize(float3(0.5, 1.0, 0.3));
    
    // Bias world position slightly along normal to avoid self-shadowing
    float3 ray_origin = world_pos + normal.xyz * 0.01;
    
    ray r;
    r.origin = ray_origin;
    r.direction = sun_dir;
    r.min_distance = 0.0;
    r.max_distance = 1000.0; // max shadow trace distance

    intersector<instancing, triangle_data> i;
    i.assume_geometry_type(geometry_type::triangle);
    
    intersection_result<instancing, triangle_data> result = i.intersect(r, tlas);

    // Basic lighting
    float n_dot_l = max(dot(normal.xyz, sun_dir), 0.0);
    float3 ambient = albedo.rgb * 0.2;
    float3 diffuse = albedo.rgb * n_dot_l;

    if (result.type != intersection_type::none) {
        // In shadow
        diffuse *= 0.1; // Darken if shadowed
    }

    float3 final_color = ambient + diffuse;
    output_texture.write(float4(final_color, 1.0), tid);
}
