#![allow(dead_code)]

#[macro_use]
mod common;

extern crate sdl2;

#[macro_use]
extern crate bitfield;

// TODO These should not be public, but for testing right now they can be
pub mod apu;
pub mod bus;
pub mod cartridge;
pub mod controller;
pub mod cpu;
pub mod instructions;
pub mod mapper;
pub mod memory;
pub mod ppu;
pub mod sdl_interface;
pub mod vnes;

pub use vnes::VNES;

pub mod graphics {
    pub use super::sdl_interface::graphics::Renderer;
    pub const FRAME_RATE_NS: u32 =
        1_000_000_000 / 60 / super::sdl_interface::graphics::NES_SCREEN_HEIGHT;
}

pub mod audio {}

pub mod input {}
