use super::Renderer;

pub struct NOPRenderer;
impl NOPRenderer {
    pub fn new() -> Self {
        NOPRenderer {}
    }
}

impl Renderer for NOPRenderer {
    fn draw_line(&mut self, _line: &[u8], _row: u32) {}
    fn draw_frame(&mut self, _buf: &[u8]) {}
}
