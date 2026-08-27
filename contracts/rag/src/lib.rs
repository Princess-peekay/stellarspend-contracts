#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

mod collections;
mod access;
mod documents;
mod embeddings;
mod verification;

pub use collections::*;
pub use access::*;
pub use documents::*;
pub use embeddings::*;
pub use verification::*;

#[contract]
pub struct RagContract;

#[contractimpl]
impl RagContract {
    pub fn init(_env: Env) {}
}