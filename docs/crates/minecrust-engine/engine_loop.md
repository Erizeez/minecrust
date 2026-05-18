# 引擎主循环设计 (Engine Game Loop)

Minecrust 的游戏循环不使用闭源黑盒，而是显式地暴露在 `minecrust-engine` 中，由 `minecrust-client` 主导调用。这样做的目的是为了完全掌控帧率与物理刷新率的解耦。

## 核心设计理念

我们采用“**固定步长更新 (Fixed Update) + 变步长渲染 (Variable Render)**” 的工业标准架构。

### 1. `winit` 事件捕获
在 `winit` 提供的 `EventLoop::run` 中，我们捕获：
- `WindowEvent::CloseRequested` -> 退出程序
- `WindowEvent::KeyboardInput` / `MouseInput` -> 缓存到专门的 Input 状态机中
- `Event::MainEventsCleared` -> 触发游戏逻辑滴答 (Tick)

### 2. Fixed Update (物理与网络层)
为了确保重力、碰撞、服务器数据同步的完全决定性（Determinism），这部分代码必须以恒定的频率执行（例如 20 TPS 或 60 TPS）。

```rust
// 累加器模式
let mut accumulator = 0.0;
let dt = 1.0 / 60.0; // 60 TPS

loop {
    let frame_time = get_frame_time();
    accumulator += frame_time;

    while accumulator >= dt {
        // --- 显式调用 ECS 系统 ---
        // 1. 处理网络收发
        network_system(&mut world);
        // 2. 玩家输入转换为速度
        input_system(&mut world);
        // 3. 物理系统步进
        physics_system(&mut world, dt);
        
        accumulator -= dt;
    }
    
    // ... 进入渲染阶段
}
```

### 3. Variable Update (渲染层)
这一层代码每帧都会执行（通常受限于屏幕的 60Hz 或 144Hz 刷新率）。
我们在这里不处理任何核心物理逻辑，只处理：
- 摄像机的平滑插值（根据 `accumulator / dt` 的比例在物理两帧之间插值，以消除抖动）。
- `wgpu` 的 RenderPass 构建与命令提交。
