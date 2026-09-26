//! Proof's Godot bridge. The engine only sees one class, [`game::Game`]; everything the
//! player sees is printed from Rust (proof_core art → proof_raster plates → riso shader).

use godot::prelude::*;

mod game;
mod riso;
mod screens;
mod sfx;
mod ui;

struct ProofExtension;

#[gdextension]
unsafe impl ExtensionLibrary for ProofExtension {}
