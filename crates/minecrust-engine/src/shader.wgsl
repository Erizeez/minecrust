// Bind Group 0: Camera
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Bind Group 1: Texture Atlas
@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var t_normal: texture_2d<f32>;
@group(1) @binding(2) var t_specular: texture_2d<f32>;
@group(1) @binding(3) var s_sampler: sampler;

// Bind Group 2: Entity Transform (Optional for chunks, required for entities)
struct EntityUniform {
    model: mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> entity: EntityUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = model.uv;
    out.color = model.color;
    
    // Apply entity model matrix
    let world_position = entity.model * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.world_pos = world_position.xyz;
    
    return out;
}

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) mrao: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    let albedo = textureSample(t_albedo, s_sampler, in.uv);
    let final_alpha = albedo.a * in.color.a;
    
    if (final_alpha < 0.1) {
        discard;
    }
    
    // Screen-door transparency (dithering) for partial transparency (e.g. water)
    if (final_alpha < 0.99) {
        let x = u32(in.clip_position.x);
        let y = u32(in.clip_position.y);
        
        // Simple 4x4 Bayer matrix approximation for dithering
        // We use clip_position.xy (screen coordinates) to generate a dither threshold
        let index = (x % 4u) + (y % 4u) * 4u;
        // Precomputed 4x4 Bayer matrix thresholds divided by 16.0
        var thresholds = array<f32, 16>(
            0.0625, 0.5625, 0.1875, 0.6875,
            0.8125, 0.3125, 0.9375, 0.4375,
            0.2500, 0.7500, 0.1250, 0.6250,
            1.0000, 0.5000, 0.8750, 0.3750
        );
        let threshold = thresholds[index];
        
        if (final_alpha < threshold) {
            discard;
        }
    }
    
    // Calculate flat face normal from world position derivatives
    let dx = dpdx(in.world_pos);
    let dy = dpdy(in.world_pos);
    let face_normal = normalize(cross(dx, dy));
    
    // Read PBR maps
    let normal_map = textureSample(t_normal, s_sampler, in.uv);
    let specular_map = textureSample(t_specular, s_sampler, in.uv);
    
    var out: GBufferOutput;
    out.albedo = vec4<f32>(albedo.rgb * in.color.rgb, 1.0);
    // Store face_normal for now (w=1.0)
    out.normal = vec4<f32>(face_normal, 1.0); 
    out.mrao = specular_map;
    return out;
}
