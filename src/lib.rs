#![allow(dead_code)]

#[macro_use]
mod common;

extern crate sdl2;

#[macro_use]
extern crate bitfield;

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod instructions;
mod mapper;
mod memory;
mod ppu;
mod sdl_interface;
mod vnes;

pub use vnes::VNES;

pub mod graphics {
    pub use super::sdl_interface::graphics::Renderer;
    pub const FRAME_RATE_NS: u32 = 1_000_000_000 / 60 / 241;
}

pub mod audio {}

pub mod input {}
