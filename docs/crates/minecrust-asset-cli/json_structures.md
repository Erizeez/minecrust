# Minecraft 资产 JSON 结构映射

为了反序列化 Minecraft 1.21.1 客户端内的资产，我们在 `minecrust-asset-cli` 中定义了以下核心 Rust 结构体。

## 1. Blockstates (`assets/minecraft/blockstates/*.json`)

Blockstate JSON 定义了方块的不同变体（如不同朝向的楼梯）与具体模型文件的映射关系。它通常包含一个 `variants` 对象，或者一个 `multipart` 数组。对于 MVP，我们只处理最简单的 `variants`。

```rust
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BlockState {
    pub variants: HashMap<String, Variant>,
}

#[derive(Debug, Deserialize)]
pub struct Variant {
    pub model: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub uvlock: bool,
}
```

## 2. Models (`assets/minecraft/models/block/*.json`)

Model JSON 定义了具体的 3D 模型拓扑结构。它包含父模型的继承关系（`parent`），纹理变量（`textures`），以及组成模型的长方体元素（`elements`）。

```rust
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Model {
    pub parent: Option<String>,
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<Element>>,
}

#[derive(Debug, Deserialize)]
pub struct Element {
    pub r#from: [f32; 3],
    pub to: [f32; 3],
    pub faces: HashMap<String, Face>,
}

#[derive(Debug, Deserialize)]
pub struct Face {
    pub texture: String,
    pub cullface: Option<String>,
}
```

## 验证逻辑
第一步的评估验证标准是：用上述结构体，能否 100% 不报错地通过 `serde_json::from_str` 将 jar 包里提取出的 `stone.json` 字符串转化为这几个 Struct 实例。
