#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct RefundsContract;

#[contractimpl]
impl RefundsContract {
    pub fn get_refund_status(_env: Env, refund_id: u64) -> Symbol {
        if refund_id == 1 {
            symbol_short!("pending")
        } else if refund_id == 2 {
            symbol_short!("processed")
        } else {
            symbol_short!("rejected")
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_get_refund_status() {
        let env = Env::default();
        let contract_id = env.register_contract(None, RefundsContract);
        let client = RefundsContractClient::new(&env, &contract_id);

        assert_eq!(client.get_refund_status(&1), symbol_short!("pending"));
        assert_eq!(client.get_refund_status(&2), symbol_short!("processed"));
        assert_eq!(client.get_refund_status(&3), symbol_short!("rejected"));
    }
}
