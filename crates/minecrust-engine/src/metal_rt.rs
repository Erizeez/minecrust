use std::sync::Arc;
use metal::*;
use objc::rc::autoreleasepool;

pub struct MetalRtContext {
    pub device: metal::Device,
    pub command_queue: metal::CommandQueue,
    pub compute_pipeline: metal::ComputePipelineState,
}

impl MetalRtContext {
    pub fn new(wgpu_device: &wgpu::Device, wgpu_queue: &wgpu::Queue) -> Self {
        // Extract raw Metal device and queue from wgpu using wgpu-hal
        // wgpu-hal API might vary slightly by version, we'll use a direct cast approach 
        // or just let wgpu create it. Actually, for metal-rs we can just grab the default device for now if we want,
        // but it's safer to get the exact one wgpu uses.
        
        unsafe {
            // In wgpu 22.0, as_hal is available.
            let mut mtl_device: Option<metal::Device> = None;
            let mut mtl_queue: Option<metal::CommandQueue> = None;
            
            wgpu_device.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_device| {
                if let Some(hal_dev) = hal_device {
                    // Extract metal device pointer
                    // In wgpu_hal, hal_dev.shared_instance() returns &metal::DeviceRef
                    mtl_device = Some(hal_dev.raw_device().lock().clone());
                }
            });

            // For wgpu 22.0, wgpu::Queue might not have as_hal. We can just create a new command queue from the device!
            mtl_queue = Some(mtl_device.as_ref().unwrap().new_command_queue());

            let device = mtl_device.expect("Failed to extract Metal Device from wgpu");
            
            // Compile Compute Shader
            let shader_src = include_str!("shaders/raytrace.metal");
            let options = metal::CompileOptions::new();
            let library = device.new_library_with_source(shader_src, &options).expect("Failed to compile raytrace.metal");
            let function = library.get_function("rt_main", None).expect("Failed to find rt_main function");
            let compute_pipeline = device.new_compute_pipeline_state_with_function(&function).expect("Failed to create compute pipeline");

            Self {
                device,
                command_queue: mtl_queue.expect("Failed to extract Metal CommandQueue from wgpu"),
                compute_pipeline,
            }
        }
    }

    pub unsafe fn extract_texture_view(view: &wgpu::TextureView) -> metal::Texture {
        let mut mtl_tex: Option<metal::Texture> = None;
        view.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_view| {
            if let Some(v) = hal_view {
                // wgpu_hal::metal::TextureView has `raw: metal::Texture` (align 8) and `aspects` (align 1).
                // Rust will place `raw` at offset 0.
                let raw_ptr = v as *const _ as *const metal::Texture;
                mtl_tex = Some((*raw_ptr).clone());
            }
        });
        mtl_tex.expect("Failed to extract Metal Texture")
    }

    pub unsafe fn extract_buffer(buf: &wgpu::Buffer) -> metal::Buffer {
        let mut mtl_buf: Option<metal::Buffer> = None;
        buf.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_buf| {
            if let Some(b) = hal_buf {
                let ptr = b as *const _ as *const u64;
                let val1 = *ptr;
                let val2 = *ptr.offset(1);
                
                // One is size (usually small, max a few GB), the other is a pointer (always > 4GB on 64-bit Mac).
                // Objective-C object pointers on macOS typically start with 0x00006... or 0x00000001...
                let raw_ptr = if val1 > 0x1_0000_0000 { val1 } else { val2 } as *mut objc::runtime::Object;
                
                // metal::Buffer is a wrapper around `Id<MTLBuffer>` which is just a pointer
                let buffer_ptr = &raw_ptr as *const _ as *const metal::Buffer;
                mtl_buf = Some((*buffer_ptr).clone());
            }
        });
        mtl_buf.expect("Failed to extract Metal Buffer")
    }

    pub fn build_blas(
        &self,
        vertex_buffer: &metal::Buffer,
        vertex_stride: u64,
        index_buffer: &metal::Buffer,
        index_count: u32,
    ) -> metal::AccelerationStructure {
        let geometry = metal::AccelerationStructureTriangleGeometryDescriptor::descriptor();
        geometry.set_vertex_buffer(Some(vertex_buffer));
        geometry.set_vertex_stride(vertex_stride);
        geometry.set_index_buffer(Some(index_buffer));
        geometry.set_index_type(metal::MTLIndexType::UInt32);
        geometry.set_triangle_count((index_count / 3) as u64);

        let geom_ref: &metal::AccelerationStructureGeometryDescriptorRef = geometry.as_ref();
        let geometry_array = metal::Array::from_slice(&[geom_ref]);
        let desc = metal::PrimitiveAccelerationStructureDescriptor::descriptor();
        desc.set_geometry_descriptors(geometry_array);

        let sizes = self.device.acceleration_structure_sizes_with_descriptor(&desc);
        let accel_struct = self.device.new_acceleration_structure_with_size(sizes.acceleration_structure_size);
        
        let scratch_buffer = self.device.new_buffer(sizes.build_scratch_buffer_size, metal::MTLResourceOptions::StorageModePrivate);

        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_acceleration_structure_command_encoder();
        encoder.build_acceleration_structure(&accel_struct, &desc, &scratch_buffer, 0);
        encoder.end_encoding();
        command_buffer.commit();
        command_buffer.wait_until_completed();

        accel_struct
    }

    pub fn build_tlas(
        &self,
        instances: &[metal::MTLAccelerationStructureInstanceDescriptor],
        blas_array: &[&metal::AccelerationStructureRef],
    ) -> metal::AccelerationStructure {
        let instance_buffer = self.device.new_buffer_with_data(
            unsafe { std::mem::transmute(instances.as_ptr()) },
            (instances.len() * std::mem::size_of::<metal::MTLAccelerationStructureInstanceDescriptor>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        let desc = metal::InstanceAccelerationStructureDescriptor::descriptor();
        desc.set_instance_descriptor_type(metal::MTLAccelerationStructureInstanceDescriptorType::Default);
        let instances_array = metal::Array::from_slice(blas_array);
        desc.set_instanced_acceleration_structures(instances_array);
        desc.set_instance_count(instances.len() as u64);
        desc.set_instance_descriptor_buffer(&instance_buffer);
        desc.set_instance_descriptor_buffer_offset(0);
        desc.set_instance_descriptor_stride(std::mem::size_of::<metal::MTLAccelerationStructureInstanceDescriptor>() as u64);

        let sizes = self.device.acceleration_structure_sizes_with_descriptor(&desc);
        let accel_struct = self.device.new_acceleration_structure_with_size(sizes.acceleration_structure_size);
        
        let scratch_buffer = self.device.new_buffer(sizes.build_scratch_buffer_size, metal::MTLResourceOptions::StorageModePrivate);

        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_acceleration_structure_command_encoder();
        encoder.build_acceleration_structure(&accel_struct, &desc, &scratch_buffer, 0);
        encoder.end_encoding();
        command_buffer.commit();
        command_buffer.wait_until_completed();

        accel_struct
    }

    pub fn dispatch(
        &self,
        output: &wgpu::TextureView,
        history: &wgpu::TextureView,
        albedo: &wgpu::TextureView,
        normal: &wgpu::TextureView,
        mrao: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera_buffer: &wgpu::Buffer,
        tlas: Option<&metal::AccelerationStructure>,
    ) {
        let mtl_output = unsafe { Self::extract_texture_view(output) };
        let mtl_history = unsafe { Self::extract_texture_view(history) };
        let mtl_albedo = unsafe { Self::extract_texture_view(albedo) };
        let mtl_normal = unsafe { Self::extract_texture_view(normal) };
        let mtl_mrao = unsafe { Self::extract_texture_view(mrao) };
        let mtl_depth = unsafe { Self::extract_texture_view(depth) };
        let mtl_camera = unsafe { Self::extract_buffer(camera_buffer) };

        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        
        encoder.set_compute_pipeline_state(&self.compute_pipeline);
        
        encoder.set_texture(0, Some(&mtl_output));
        encoder.set_texture(1, Some(&mtl_albedo));
        encoder.set_texture(2, Some(&mtl_normal));
        encoder.set_texture(3, Some(&mtl_mrao));
        encoder.set_texture(4, Some(&mtl_depth));
        encoder.set_texture(5, Some(&mtl_history));
        encoder.set_buffer(6, Some(&mtl_camera), 0);

        if let Some(accel) = tlas {
            encoder.set_acceleration_structure(7, Some(accel.as_ref()));
        }

        let width = mtl_output.width();
        let height = mtl_output.height();
        
        let threadgroup_size = metal::MTLSize { width: 16, height: 16, depth: 1 };
        let grid_size = metal::MTLSize { width, height, depth: 1 };
        
        encoder.dispatch_threads(grid_size, threadgroup_size);
        encoder.end_encoding();
        
        command_buffer.commit();
    }
}
