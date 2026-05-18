# 资产预处理管线 (Asset Pipeline)

为了实现极致的客户端启动速度和运行时性能，Minecrust 不会在游戏运行时去解析繁杂的 Minecraft JSON 文件和散落的 PNG 图片。相反，我们采用“离线烘焙”（Offline Baking）策略。

## `minecrust-asset-cli` 工具
这是一个独立的 Rust CLI 工程，位于 `crates/minecrust-asset-cli`。它的主要职责是将原版的资源打包转换为我们的自定义格式。

### 处理流程 (Workflow)

1. **输入阶段**:
   用户提供原版游戏的路径，例如 `~/.minecraft/versions/1.21.1/1.21.1.jar`。工具会将此 jar 包（本质上是 zip）解压到内存或临时目录。

2. **解析阶段**:
   - **材质 (Textures)**: 扫描所有 `assets/minecraft/textures/block/` 下的 PNG。
   - **模型 (Models)**: 递归解析 `assets/minecraft/models/`，将 JSON 模型（例如由 `elements` 和 `faces` 组成的立方体）转换为内存中的网格拓扑。
   - **方块状态 (Blockstates)**: 解析 `assets/minecraft/blockstates/`，确定特定方块的变体（Variant）应该使用哪个模型。例如，朝向不同方向的原木使用不同的模型旋转。

3. **烘焙阶段 (Baking)**:
   - **纹理图集 (Texture Atlas)**: 将所有小方块的材质拼接成一张巨大的 2D 纹理贴图（例如 2048x2048）。在此过程中，记录每一个小材质在图集中的 UV 坐标。
   - **网格预计算**: 针对各种方块模型（标准方块、台阶、楼梯、草丛十字模型），提前生成好顶点相对坐标和 UV 映射，以适应 wgpu 的顶点缓冲区。

4. **输出阶段**:
   将所有处理后的数据打包成单个二进制文件，例如 `assets.mca` (Minecrust Asset)。这个文件内部包含：
   - 文件头：版本号和索引偏移量。
   - Block Dictionary：一个巨大的数组，根据全局 Block ID（如 `minecraft:stone` ID=1）索引到它的六个面的渲染配置（所用模型的指针、UV 坐标）。
   - Texture Atlas：一张合并后的 PNG（或直接压缩为 GPU 友好的格式如 BC7/ASTC）。

## 客户端运行时的资产加载
由于我们在离线阶段已经将所有复杂的 JSON 逻辑拍平，客户端启动时只需执行以下操作：
1. 读取并解压 `.mca` 二进制文件。
2. 将 Texture Atlas 直接上传到 `wgpu` 的 `Texture` 显存中。
3. 将 Block Dictionary 载入内存，供贪婪网格（Greedy Meshing）算法在渲染区块时快速查询。

这一策略彻底解耦了原版复杂的资产定义，确保我们的 `wgpu` 客户端保持极简和高效。
