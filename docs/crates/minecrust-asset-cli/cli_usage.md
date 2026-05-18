# `minecrust-asset-cli` 使用指南

`minecrust-asset-cli` 是 Minecrust 项目的离线资产烘焙工具。它的目标是读取原始的 Minecraft 客户端 `.jar` 文件，解析其中的 JSON 资产，并最终将其打包为适合 `wgpu` 高效加载的二进制格式。

## 第一阶段功能 (Phase 2 MVP)

在当前阶段，该 CLI 仅具有 **提取并验证 (Extract & Verify)** 的功能，主要用于验证我们的 Rust 结构体是否能够正确反序列化原版 JSON。

### 命令用法

```bash
cargo run --bin minecrust-asset-cli -- extract --jar-path <PATH> --out-dir <DIR>
```

**参数说明**:
- `extract`: 子命令，表示我们要进行资产解析与提取。
- `--jar-path` 或 `-j`: 必填。指向原版客户端 jar 包的路径。例如 `../../assets/raw/1.21.1.jar`。
- `--out-dir` 或 `-o`: 可选。解析后的缓存输出目录。默认值为当前目录下的 `assets/processed/`。

### 预期输出
程序会打开指定的 ZIP 包，在控制台中打印出如下调试信息：
1. 成功读取 JAR 包的元数据。
2. 找到 `assets/minecraft/blockstates/stone.json`，并将其打印为反序列化后的 Rust 结构。
3. 找到 `assets/minecraft/models/block/stone.json` 或 `cube_all.json`，打印出包含其 `textures` 贴图路径的结构。
