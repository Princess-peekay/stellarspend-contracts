#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

mod collections;
mod access;
mod documents;
mod verification;
mod embeddings;

pub use collections::*;
pub use access::*;
pub use documents::*;
pub use verification::*;
pub use embeddings::*;

#[contract]
pub struct RagContract;

#[contractimpl]
impl RagContract {
    pub fn init(_env: Env) {}
}