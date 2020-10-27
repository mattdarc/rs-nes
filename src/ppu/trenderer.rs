use venus::Renderer;

fn main() {
    let mut renderer = Renderer::new();
    if let Err(e) = renderer.init("TestRenderer") {
	panic!("{}", e);
    }
}
