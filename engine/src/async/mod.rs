pub struct FrameIndex {
    current: usize,
    count: usize,
}

impl FrameIndex {
    pub fn new(count: usize) -> Self {
        Self { current: 0, count }
    }

    pub fn index(&self) -> usize {
        self.current
    }

    pub fn advance(&mut self) {
        self.current = (self.current + 1) % self.count;
    }
}
