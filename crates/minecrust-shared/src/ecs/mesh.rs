/// Component attached to an entity to mark it for rendering with a specific mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mesh {
    /// The identifier for the mesh to be rendered (e.g. "steve_head").
    pub mesh_id: String,
    pub visible: bool,
}

impl Mesh {
    pub fn new(mesh_id: impl Into<String>) -> Self {
        Self {
            mesh_id: mesh_id.into(),
            visible: true,
        }
    }
}
