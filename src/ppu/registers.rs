#![allow(non_snake_case)]
bitflags! {
    #[derive(Default)]
    pub struct PpuCtrl: u8 {
        const NMI_ENABLE        = 0x80;
        const SLAVE_SELECT      = 0x40;
        const SPRITE_SIZE       = 0x20;
        const BG_TABLE_ADDR     = 0x10;
        const SPRITE_TABLE_ADDR = 0x08;
        const VRAM_INCR         = 0x04;
        const NAMETABLE_ADDR    = 0x03;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct PpuMask: u8 {
        const EMPH_BLUE = 0x80;
        const EMPH_GREEN = 0x40;
        const EMPH_RED   = 0x20;
        const SHOW_SPRITES = 0x10;
        const SHOW_BG = 0x08;
        const SHOW_LEFT_SPRITES = 0x04;
        const SHOW_LEFT_BG = 0x02;
        const GRAYSCALE = 0x01;

    }
}

bitflags! {
    #[derive(Default)]
    pub struct PpuStatus: u8 {
        const VBLANK_STARTED = 0x80;
        const SPRITE_0_HIT = 0x40;
        const SPRITE_OVERFLOW = 0x20;
        const PREV_LSB = 0x1F;
    }
}

#[derive(Default)]
pub struct Registers {
    pub ctrl: PpuCtrl,
    pub mask: PpuMask,
    pub status: PpuStatus,
    pub oamaddr: u8,
    pub oamdata: u8,
    pub scroll: u8,
    pub addr: u8,
    pub data: u8,
}
