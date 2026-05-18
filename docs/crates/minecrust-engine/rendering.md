# 渲染管线设计 (wgpu Rendering Pipeline)

Minecrust 的渲染管线专门为海量体素（Voxels）的极速绘制而生。它放弃了传统引擎中复杂的层级对象树（Scene Graph），直接将内存中扁平的 ECS 组件数据冲入显存。

## MVP WGSL Shader 设计

第一阶段 MVP（最简可行产品）我们只追求把带有材质的方块显示在屏幕上，没有复杂的光照。

顶点数据结构（Vertex Buffer Layout）：
```rust
struct Vertex {
    position: [f32; 3], // 顶点局部坐标
    uv: [f32; 2],       // 对应 texture atlas 上的精确坐标
}
```

我们的 `WGSL` 着色器非常直接：
- **Vertex Shader**: 获取顶点局部坐标，乘以 `Transform` 组件带来的模型矩阵（Model Matrix），再乘以 BindGroup0 里的 `ViewProjection` 矩阵。
- **Fragment Shader**: 根据传递过来的 UV 坐标，直接去 BindGroup1 里的 `Texture Atlas` 采样颜色输出。如果是透明像素则 `discard`。

## wgpu 核心抽象层 (`minecrust-engine::renderer`)

为了让 `minecrust-client` 的开发者写起业务代码来像呼吸一样自然，引擎库会抽象出一个核心的 `Renderer` 结构体。

### 1. 资源管理与绑定组 (Bind Groups)
`Renderer` 内部会维护两个关键的资源组：
- **Camera Uniform Bind Group**: 随时可以通过 `update_camera(view, proj)` 更新显存里的矩阵，引擎会自动处理底层的 Buffer 上传。
- **Atlas Texture Bind Group**: 在客户端启动并解析出 `assets.mca` 后，调用 `load_texture_atlas(&bytes)`，引擎自动创建 `wgpu::Texture` 并生成 Sampler，绑定到 `group(1)`。

### 2. 画布绘制接口 (Draw API)
在每一帧的渲染循环中，使用者只需这样调用：

```rust
// 开始一个 Render Pass
let mut pass = renderer.begin_pass();

// 应用全局上下文
pass.set_camera();
pass.set_atlas();

// 从 ECS 中找出所有需要渲染的区块网格或实体网格进行绘制
for (transform, mesh) in world.query::<(&Transform, &GpuMesh)>() {
    pass.draw_mesh(mesh, transform.to_matrix());
}

// 提交队列
renderer.submit();
```

通过这一套极简但高效的抽象，我们既保持了 `wgpu` 的极致性能，又让上层的体素生成逻辑和 ECS 系统不必接触底层繁琐的 Vulkan/Metal API 概念。
