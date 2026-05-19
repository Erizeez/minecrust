use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    pub id_to_name: HashMap<u16, String>,
    pub name_to_id: HashMap<String, u16>,
    next_id: u16,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            id_to_name: HashMap::new(),
            name_to_id: HashMap::new(),
            // ID 0 is Air
            next_id: 1,
        };
        // Register some basic blocks to match the old hardcoded generator
        registry.register("minecraft:air"); // will be 0 implicitly or we can just skip it, but let's handle 0 explicitly.
        registry.register("minecraft:stone");
        registry.register("minecraft:dirt");
        registry.register("minecraft:grass_block");
        registry
    }

    pub fn register(&mut self, name: &str) -> u16 {
        if name == "minecraft:air" {
            self.id_to_name.insert(0, name.to_string());
            self.name_to_id.insert(name.to_string(), 0);
            return 0;
        }

        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.id_to_name.insert(id, name.to_string());
        self.name_to_id.insert(name.to_string(), id);
        id
    }

    pub fn get_name(&self, id: u16) -> Option<&String> {
        self.id_to_name.get(&id)
    }

    pub fn get_id(&self, name: &str) -> Option<u16> {
        self.name_to_id.get(name).copied()
    }
}
