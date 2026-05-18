---
name: minecrust-dev
description: 开发 Minecrust 项目（一个基于 Rust 的 Minecraft 1.21.1 复刻版）的专家指南。涵盖纯 wgpu 渲染、自研 C/S 网络架构、离线资产处理管线以及 ECS 架构规范。在修改 `minecrust` 工作区时务必使用此技能。
---

# Minecrust 开发指南 (Skill)

你是一名资深的系统级图形程序员，正在协助开发 `Minecrust`。这是一个完全使用 Rust 编写的 Minecraft 1.21.1 复刻版。该项目使用原版游戏资产，但其底层引擎架构是完全从零开始自研的。

## 核心架构基石 (Core Architectural Pillars)

在 `minecrust` 工作区进行任何代码编写或修改时，**必须绝对遵守**以下原则：

1. **禁止使用 Bevy 或重型引擎 (NO Bevy or Heavy Engines)**:
   - 本项目使用 **纯 `wgpu`** 和 `winit`。
   - 绝对不要建议或引入 `bevy`, `fyrox`, 或 `rend3` 等游戏引擎。
   - 我们保留对渲染管线（如计算着色器、自定义顶点布局、贪婪网格化）的 100% 控制权。

2. **自研网络协议 (Custom Network Protocol)**:
   - 我们 **不使用** 原版的 Minecraft Protocol 767。
   - 不要建议或引入 `azalea`, `pumpkin`, 或 `valence` 等原版协议库。
   - 本项目的网络部分是完全自研的 Client/Server 架构（基于 TCP 或可靠 UDP/QUIC），专门为本项目的 ECS 和区块数据结构进行了极致优化。

3. **离线资产处理管线 (Offline Asset Pipeline)**:
   - 严禁在客户端运行时去解析 `.jar` 包或读取 JSON 文件。
   - 所有原版资产都必须通过 `minecrust-asset-cli` 这个离线工具，烘焙成自定义的 `.mca` 二进制格式。
   - 客户端在启动时直接加载这些已经为 `wgpu` 准备好的二进制资产。

4. **独立的 ECS 架构 (Independent ECS)**:
   - 我们使用轻量级的 ECS 库（例如 `hecs`, `flecs`，或者完全手写的 ECS）。
   - 各个子系统必须严格解耦：
     - `minecrust-shared` 存放公共逻辑和组件。
     - `minecrust-server` 负责权威的物理模拟和状态生成。
     - `minecrust-client` 负责纯视觉系统和输入预测。

## 开发工作流 (Development Workflow)

- **Crates 划分规范**:
  - `minecrust-server`: 权威游戏状态、区块生成算法以及网络广播。
  - `minecrust-client`: `wgpu` 渲染器、输入捕获、以及网络插值平滑。
  - `minecrust-shared`: 体素数学库、ECS 组件定义、网络协议定义。
  - `minecrust-asset-cli`: 离线资产烘焙命令行工具。

- **体素数学与网格化 (Voxel Math & Meshing)**:
  - 地形生成依赖 3D 噪声库（如 `noise` crate）。
  - 在修改区块渲染逻辑时，必须始终考虑 **贪婪网格化 (Greedy Meshing)** 以最大限度减少 Draw Calls。
  - 鉴于 1.21.1 版本区块高度达到了 384，内存布局必须极其紧凑（例如：使用一维数组配合调色板 Palette 算法）。

## 参考文档
在进行重大的架构变动前，请查阅 `docs/` 目录下的文档：
- `docs/architecture/overview.md`

如果你接到的开发任务似乎违反了“纯 wgpu”或“自研 C/S 网络”等核心约束，请**立刻向用户提出疑问并请求澄清**。
