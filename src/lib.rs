#[macro_use]
mod common;

extern crate sdl2; 

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod instructions;
mod mapper;
mod memory;
mod ppu;
mod vnes;

pub use vnes::VNES as VNES;

pub mod graphics {
    pub use super::ppu::Renderer as Renderer;
    pub use super::ppu::sdl_interface::Texture as Texture;
    pub use super::ppu::sdl_interface::Coordinates as Coordinates;
    pub const FRAME_RATE_NS: u32 = 1_000_000_000 / 60;
}
