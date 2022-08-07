use super::Renderer;

pub struct NOPRenderer;
impl NOPRenderer {
    pub fn new() -> Self {
        NOPRenderer {}
    }
}

impl Renderer for NOPRenderer {
    fn render_line(&mut self, _line: &[u8], _row: u32) {}
    fn render_frame(&mut self, _buf: &[u8], _width: u32, _height: u32) {}
}
