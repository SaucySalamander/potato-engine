pub struct DrawCommand {
    pub entity_id: u32,
}

pub struct DrawQueue {
    pub opaque: Vec<DrawCommand>
}

impl DrawQueue {
    pub fn new() -> Self {
        Self { opaque: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.opaque.clear();
    }
}
