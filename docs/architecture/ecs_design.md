# 实体组件系统 (ECS) 设计

Minecrust 在服务端和客户端都高度依赖数据驱动的设计理念，以应对体素游戏中大量动态实体和高频状态更新的需求。由于我们不使用庞大的 Bevy 框架，ECS 模块（基于 `hecs` 或自定义实现）的设计必须非常严谨。

## 核心设计原则

1. **组合优于继承**: 游戏世界中的一切“动态”事物（玩家、掉落物、投掷物）都是一个没有任何固有逻辑的 `Entity` (ID)。它们的行为完全由身上挂载的 `Component` (组件) 和后台运行的 `System` (系统) 决定。
2. **纯数据组件**: 组件应当仅包含数据（Plain Old Data），绝对不要把逻辑（方法）写在组件里。逻辑全部归属于独立的 System 函数。
3. **严格的内存布局**: 为了最大化 CPU 缓存命中率，组件的数据结构必须尽可能紧凑，内存对齐（Memory Alignment）也是重点考虑对象。

## 核心组件 (Components) 定义草案

在 `minecrust-shared` 包中，我们会定义以下跨 C/S 两端共用的核心组件：

- `Transform`: 包含 `Vec3` 位置和四元数旋转。所有可见/可碰撞的实体必备。
- `Velocity`: 包含 `Vec3` 的当前速度向量，用于物理步进计算。
- `AABB` (Axis-Aligned Bounding Box): 轴对齐包围盒，记录宽度和高度，用于与地形或其他实体的碰撞检测。
- `Player`: 标记组件，表示这是一个玩家。包含玩家名字、UUID 和库存。

客户端独有组件（仅在 `minecrust-client` 中存在）：
- `Camera`: 挂载在本地玩家实体上，用于计算视图矩阵 (View Matrix)。
- `Interpolation`: 记录上一次收到服务端状态和当前时间，用于客户端的平滑插值渲染。
- `MeshRef`: 指向 `wgpu` 的网格显存缓冲区句柄。

服务端独有组件（仅在 `minecrust-server` 中存在）：
- `ClientConnection`: 绑定网络流（Socket），处理接收和发送的具体网络数据包。
- `Gravity`: 标记实体受重力影响。

## 系统 (Systems) 的调度

系统的调度分为两个主要世界：

1. **Server World**:
   - `network_recv_system`: 接收来自玩家的输入。
   - `physics_system`: 遍历所有拥有 `Transform + Velocity + AABB` 的实体，计算移动、重力和与地形的碰撞。
   - `chunk_generation_system`: 根据玩家位置调度新区块的生成。
   - `network_send_system`: 将更新后的实体状态打包发送给客户端。

2. **Client World**:
   - `input_system`: 读取键鼠输入，更新本地玩家的 `Velocity` 或视角，并发送输入包。
   - `prediction_system`: 先行在本地执行简化的物理模拟以实现即时反馈。
   - `meshing_system`: 在后台线程中利用贪婪网格算法，将下载到的 Chunk 数据生成渲染所需的顶点数组。
   - `render_system`: 遍历所有带有 `Transform + MeshRef` 的实体，通过 `wgpu` 发起 Draw Call。

这种严格解耦的 ECS 架构，确保了我们自研代码的逻辑清晰、极其高效并且方便随时向多线程扩展。
