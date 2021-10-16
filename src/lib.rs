#![allow(warnings)]

mod errors;
mod utils;

pub mod entrypoint;
pub mod instruction;
pub mod processor;

/// Prefix used in PDA derivations to avoid collisions with other programs.
pub const PREFIX: &str = "sub";

solana_program::declare_id!("JAaJhnfYAeEjKTtKs5iBJwU11x1Hq4NtmehhCYHb2JT2");
