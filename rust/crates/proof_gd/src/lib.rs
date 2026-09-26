use godot::prelude::*;

struct ProofExtension;

#[gdextension]
unsafe impl ExtensionLibrary for ProofExtension {}

#[derive(GodotClass)]
#[class(base=Node, init)]
struct Hello {
    base: Base<Node>,
}

#[godot_api]
impl INode for Hello {
    fn ready(&mut self) {
        godot_print!("proof extension alive");
    }
}
