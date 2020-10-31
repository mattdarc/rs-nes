#[derive(Copy, Clone, Debug)]
pub struct SpriteSize(u8);
impl SpriteSize {
    fn size(self) -> (u8, u8) {
        assert!(self.0 == 8 || self.0 == 16);
        (8, self.0)
    }
}

pub struct LargeSprite {
}

pub struct SmallSprite {
}

impl LargeSprite {
    pub const SIZE: SpriteSize = SpriteSize(16);
}

impl SmallSprite {
    pub const SIZE: SpriteSize = SpriteSize(8);
}

pub enum Sprite {
    Large(LargeSprite),
    Small(SmallSprite),
}

