use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct BlockState {
    pub variants: HashMap<String, VariantList>,
}

#[derive(Debug, PartialEq)]
pub struct VariantList(pub Vec<Variant>);

impl<'de> Deserialize<'de> for VariantList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Enum {
            Single(Variant),
            Multiple(Vec<Variant>),
        }
        match Enum::deserialize(deserializer)? {
            Enum::Single(v) => Ok(VariantList(vec![v])),
            Enum::Multiple(v) => Ok(VariantList(v)),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Variant {
    pub model: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub uvlock: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Model {
    pub parent: Option<String>,
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<Element>>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Element {
    pub r#from: [f32; 3],
    pub to: [f32; 3],
    pub faces: HashMap<String, Face>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Face {
    pub texture: String,
    pub cullface: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_blockstate() {
        let json_data = r#"{
            "variants": {
                "": { "model": "minecraft:block/stone" }
            }
        }"#;

        let state: BlockState = serde_json::from_str(json_data).unwrap();
        assert_eq!(state.variants.len(), 1);
        assert_eq!(state.variants[""].0[0].model, "minecraft:block/stone");
    }

    #[test]
    fn test_parse_cube_all_model() {
        let json_data = r#"{
            "parent": "minecraft:block/cube",
            "textures": {
                "all": "minecraft:block/stone"
            }
        }"#;

        let model: Model = serde_json::from_str(json_data).unwrap();
        assert_eq!(model.parent.unwrap(), "minecraft:block/cube");
        assert_eq!(model.textures.unwrap()["all"], "minecraft:block/stone");
    }
}
