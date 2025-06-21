#[derive(Debug)]
pub struct InputState {
    pub key_w: bool,
    pub key_s: bool,
    pub key_a: bool,
    pub key_d: bool,
    pub key_space: bool,
    pub key_ctrl: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            key_w: false,
            key_s: false,
            key_a: false,
            key_d: false,
            key_space: false,
            key_ctrl: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
        }
    }
}
