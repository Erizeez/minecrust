# Minecrust 项目重构与长期优化追踪文档

为了保证 Minecrust 项目的长期可维护性、高执行效率、跨平台稳定性以及代码架构优雅性，特制定此重构方案并对暴露出的架构痛点进行长期跟踪和落地记录。

---

## 核心痛点追踪清单

### 痛点一：跨平台渲染缺陷（非 macOS 平台黑屏）
* **现状**：
  在 `minecrust-engine/src/renderer.rs` 中，场景通过延迟渲染（G-Buffer）写入贴图后，被强制分支路由至 `#[cfg(target_os = "macos")]` 中，调用基于 `metal-rs` 驱动的 Metal 硬件光追加密计算并输出至 `final_rt_output_tex`，最后全屏拷贝到 Surface 呈现。
* **缺陷**：
  在 Windows 或 Linux 平台运行时，`metal_rt_ctx` 为 `None`，导致最终呈现到交换链的贴图未进行任何像素写入，直接显示一片漆黑，渲染回退机制完全缺失。
* **重构路线**：
  设计跨平台的通用光照合成着色器（Deferred Shading Rasterization Pass / WebGPU 跨平台标准 Compute Shader 光照解算器）。当硬件光追不可用时，自动降级至基础光照着色，保证 Windows/Linux 的渲染兼容性。
* **当前状态**：`[已完成]` (Completed)

---

### 痛点二：Unsafe 内存硬编码硬换底层句柄
* **现状**：
  在 `minecrust-engine/src/metal_rt.rs` 中，为直接调用 macOS 原生 Metal API，代码强行将 `wgpu::Buffer` 或 `TextureView` 地址做 Unsafe 指针偏移，强制转换成 `metal-rs` 提供的 Metal 指针结构体：
  ```rust
  let raw_ptr = if val1 > 0x1_0000_0000 { val1 } else { val2 } as *mut objc::runtime::Object;
  let buffer_ptr = &raw_ptr as *const _ as *const metal::Buffer;
  ```
* **缺陷**：
  深度绑定了 `wgpu-hal` 未公开的内部私有内存结构，属于强耦合反模式。一旦升级 `wgpu` 依赖版本或底层字段对齐调整，这里将发生灾难性的段错误崩溃。
* **重构路线**：
  1. 将 macOS 特异的光线追踪机制抽取为独立的编译 Feature（如 `rt-metal`）并将其移入外部桥接子模块。
  2. 使用标准的 `wgpu` 跨平台计算框架，避免底层指针侵入核心公共逻辑。
* **当前状态**：`[已完成]` (Completed)

---

### 痛点三：网络实体（Remote Players）每帧重建 Mesh 与显存抖动（第一阶段重构目标）
* **现状**：
  当收到其他玩家移动的 `ServerMessage::PlayerMoved` 消息时，客户端 `GameSession::update` 会直接重置 `player.mesh = None;`。随后，在主渲染更新中，客户端每一帧都在 CPU 重新构建玩家的骨骼顶点，并调用 `create_render_mesh` 重建显存 Buffer、重新上传顶点，甚至在 macOS 下每帧重建 Metal 的 BLAS 底层光追加速结构。
* **缺陷**：
  造成灾难性的 CPU/GPU 传输开销与严重的显卡 API 拥堵，是联机多人卡顿 of 元凶。
* **重构路线**：
  - **网格静态化**：只在客户端启动资产加载后创建一次原点（`glam::Vec3::ZERO`）的共享 Steve 和 Alex 静态网格。
  - **GPU 动态变换**：利用已定义好的 Dynamic Uniform Buffer (`entity_buffer`)，在渲染时仅根据 remote player 的 position 计算平移 `glam::Mat4`，在 Vertex Shader 自动做空间变换，彻底终结 CPU 每帧计算与显存重建。
* **当前状态**：`[已完成]` (Completed)

---

### 痛点四：C/S 两端独立 ECS 世界的状态冗余与同步缺陷
* **现状**：
  目前客户端和服务端各有一个 `WorldManager`（各自独立装载 `hecs::World`），但在物理系统（`player_movement_system`）中，客户端自己在本地跑碰撞和重力模拟，并将位置发给服务端。
* **缺陷**：
  两端高度割裂。单机模式下对区块进行了无谓的双倍存储和 bincode 跨线程序列化传输；联机模式下缺少由服务器判定、客户端预测加差值回滚（Reconciliation）的标准 authoritative 网络物理模型。
* **重构路线**：
  优化 `minecrust-shared` 中的物理系统与 `ecs` 规范，建立主从端物理校验，单机模式下使用内存直传机制。
* **当前状态**：`[已完成]` (Completed)

---

### 痛点五：硬编码资源的防错设计
* **现状**：
  材质和字体的加载使用的是代码内部的硬编码相对路径（如 `"assets/raw/font/unifont.ttf"`）。
* **缺陷**：
  对于路径变更没有弹性和冗余保护，发布或打成独立包时极易触发崩溃。
* **重构路线**：
  规范化资源目录映射，引入更具弹性的配置驱动寻址。
* **当前状态**：`[已完成]` (Completed)

---

## 优化记录与追踪

### [第一阶段] 2026-05-20: 静态共享玩家 Mesh 与动态 Gpu Model Matrix 渲染
- **重构范围**：`minecrust-client` 模块下的 `game.rs` 与 `app.rs`。
- **优化效果**：
  1. 彻底终结多人运动时频繁的 `build_steve_vertices` 计算，使玩家坐标变动时的 CPU 开销降为近乎于零。
  2. 彻底终结多人移动时频繁申请/销毁 `wgpu` 顶点缓冲区、索引缓冲区，解决显存抖动（VRAM Fluctuation）。
  3. 彻底消除多人同屏移动时 GPU 重建 BLAS 加速结构的管线停顿（GPU Stalls），大幅提升多人同屏时的 FPS。

### [第二阶段] 2026-05-20: 跨平台延迟着色回退管线与 Unsafe RT Feature 物理隔离
- **重构范围**：`minecrust-engine` 模块。
- **优化效果**：
  1. **跨平台渲染兼容**：实现全平台标准的 WGSL 延迟着色回退管线。当硬件不支持光追或手动关闭时，客户端自动且流畅地降级至通用 Deferred Pass，不仅没有黑屏，且在 WGSL 渲染出随太阳高度平滑渐变的精美昼夜天空和 PBR 地表漫反射。
  2. **Unsafe RT 物理隔离**：引入编译 Feature `rt-metal`。当在非 macOS 平台或禁用默认特性时，强行转换 objc 内存对象的 Unsafe 模块在物理上完全不参与编译，彻底隔离因 wgpu 升级导致段错误的风险。

### [第三阶段] 2026-05-20: C/S 独立 ECS 物理与同步校验及 Arc 区块内存直传
- **重构范围**：`minecrust-shared`、`minecrust-server`、`minecrust-engine`。
- **优化效果**：
  1. **物理引擎下沉共享**：将 AABB、PhysicsManager 物理引擎模块解耦并下沉至无图形依赖的 `minecrust-shared`，剥离了对客户端 ChunkManager 的耦合，改为面向 Arc<Chunk> 映射访问，从而保证两端在预测与拉回校验时拥有完全一致的碰撞箱规则和物理常量。
  2. **服务端权威物理校验**：重构服务端接收 `PlayerMove` 的机制，运用公共物理引擎在服务端对玩家轨迹进行 Swept AABB 碰撞校验。当发现非法偏移（偏差 > `1e-4`）或作弊穿墙时，服务端下发 `PlayerPosAck` 强制纠正并拉回，确立了服务器物理权威。
  3. **区块内存直传与零拷贝**：在网络包序列化中开启 serde 的 `rc` 特性，使单机模式下客户端和服务端能直接通过 `Arc<Chunk>` 传递区块数据。这成功省去了内存中区块数据的重复拷贝以及频繁的高并发二进制序列化与反序列化，极大降低了单机世界加载与渲染的内存、CPU 开销。

### [第四阶段] 2026-05-20: 弹性资源路径自适应寻址与致命闪退防错自诊断
- **重构范围**：`minecrust-client` 模块下的 `asset_loader.rs` 与 `app.rs`。
- **优化效果**：
  1. **多级自适应探测机制**：重构了 `AssetLoader`，使其能动态自适应检索资源根目录。通过“环境变量覆盖探测 -> 可执行文件路径往上5级逐层回溯探测 -> 进程当前工作目录探测”的三级检索策略，彻底打破了必须在项目根目录启动的限制。
  2. **硬编码相对路径清理**：全面重构了 `app.rs` 中关于 MCA 文件、默认英文字体、GNU Unifont 中文字体以及 3 处背景音乐文件的硬编码相对路径加载，全部统一成基于 `assets_root` 的安全绝对路径拼接，确保了在任意启动路径下的 100% 稳定性。
  3. **友好的致命崩溃自诊断**：在所有检索层级都失效时，会拦截原本冷漠的系统 I/O 报错并抛出极其详尽友好的终端 Panic 自诊断输出（明确指出了检索过的所有路径、建议修复方案和环境变量手动配置方法），大幅提升了开发与独立发布时的系统鲁棒性。


