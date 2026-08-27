#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

mod collections;
mod access;
mod documents;

pub use collections::*;
pub use access::*;
pub use documents::*;

#[contract]
pub struct RagContract;

#[contractimpl]
impl RagContract {
    pub fn init(_env: Env) {}
}
