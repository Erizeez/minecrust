use crate::models::Model;
use std::collections::HashMap;

pub struct ModelResolver {
    raw_models: HashMap<String, Model>,
}

impl ModelResolver {
    pub fn new() -> Self {
        Self {
            raw_models: HashMap::new(),
        }
    }

    pub fn insert_model(&mut self, name: String, model: Model) {
        self.raw_models.insert(name, model);
    }

    pub fn resolve_textures(&self, model_name: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        self.resolve_textures_recursive(model_name, &mut result);
        
        for _ in 0..10 {
            let mut changed = false;
            let current_map = result.clone();
            for (k, v) in result.iter_mut() {
                // To suppress unused warning, actually use `k` or just ignore it.
                // We don't need `k` for this loop since we just update `v`.
                let _ = k;
                
                if v.starts_with('#') {
                    let var_name = &v[1..];
                    if let Some(resolved_val) = current_map.get(var_name) {
                        if !resolved_val.starts_with('#') {
                            *v = resolved_val.clone();
                            changed = true;
                        }
                    }
                }
            }
            if !changed { break; }
        }

        result
    }

    fn resolve_textures_recursive(&self, model_name: &str, acc: &mut HashMap<String, String>) {
        if let Some(model) = self.raw_models.get(model_name) {
            if let Some(parent) = &model.parent {
                let parent_name = if parent.starts_with("minecraft:") {
                    parent.clone()
                } else {
                    format!("minecraft:{}", parent)
                };
                self.resolve_textures_recursive(&parent_name, acc);
            }
            
            if let Some(textures) = &model.textures {
                for (k, v) in textures {
                    acc.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_inheritance_resolver() {
        let mut resolver = ModelResolver::new();
        
        let cube_json = "{\"textures\": {\"particle\": \"#down\", \"down\": \"#down\", \"up\": \"#up\", \"north\": \"#north\", \"east\": \"#east\", \"south\": \"#south\", \"west\": \"#west\"}}";
        resolver.insert_model("minecraft:block/cube".to_string(), serde_json::from_str(cube_json).unwrap());

        let cube_all_json = "{\"parent\": \"minecraft:block/cube\", \"textures\": {\"particle\": \"#all\", \"down\": \"#all\", \"up\": \"#all\", \"north\": \"#all\", \"east\": \"#all\", \"south\": \"#all\", \"west\": \"#all\"}}";
        resolver.insert_model("minecraft:block/cube_all".to_string(), serde_json::from_str(cube_all_json).unwrap());

        let stone_json = "{\"parent\": \"minecraft:block/cube_all\", \"textures\": {\"all\": \"minecraft:block/stone\"}}";
        resolver.insert_model("minecraft:block/stone".to_string(), serde_json::from_str(stone_json).unwrap());

        let resolved = resolver.resolve_textures("minecraft:block/stone");
        
        assert_eq!(resolved.get("up").unwrap(), "minecraft:block/stone");
        assert_eq!(resolved.get("north").unwrap(), "minecraft:block/stone");
    }
}
