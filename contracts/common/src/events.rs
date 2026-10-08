use crate::types::*;
use soroban_sdk::{symbol_short, Address, Env, String};

/// Event emission utilities for off-chain indexing
/// Token events
pub struct TokenEvents;

impl TokenEvents {
    pub fn mint(env: &Env, to: Address, amount: i128) {
        env.events().publish((symbol_short!("mint"),), (to, amount));
    }

    pub fn burn(env: &Env, from: Address, amount: i128) {
        env.events()
            .publish((symbol_short!("burn"),), (from, amount));
    }

    pub fn transfer(env: &Env, from: Address, to: Address, amount: i128) {
        env.events()
            .publish((symbol_short!("transfer"),), (from, to, amount));
    }

    pub fn approve(env: &Env, owner: Address, spender: Address, amount: i128) {
        env.events()
            .publish((symbol_short!("approve"),), (owner, spender, amount));
    }
}

/// Collector events
pub struct CollectorEvents;

impl CollectorEvents {
    pub fn registered(env: &Env, collector: Address, name: String) {
        env.events()
            .publish((symbol_short!("reg"),), (collector, name));
    }

    pub fn status_updated(env: &Env, collector: Address, status: CollectorStatus) {
        env.events()
            .publish((symbol_short!("status"),), (collector, status));
    }
}

/// Collection point events
pub struct CollectionPointEvents;

impl CollectionPointEvents {
    pub fn registered(env: &Env, point_id: u64, owner: Address) {
        env.events()
            .publish((symbol_short!("pt_reg"),), (point_id, owner));
    }

    pub fn verified(env: &Env, point_id: u64) {
        env.events().publish((symbol_short!("pt_ver"),), point_id);
    }
}

/// Transaction events
pub struct TransactionEvents;

impl TransactionEvents {
    pub fn recorded(
        env: &Env,
        transaction_id: u64,
        collector: Address,
        material_type: MaterialType,
        weight: u64,
    ) {
        env.events().publish(
            (symbol_short!("tx_rec"),),
            (transaction_id, collector, material_type, weight),
        );
    }

    pub fn verified(env: &Env, transaction_id: u64) {
        env.events()
            .publish((symbol_short!("tx_ver"),), transaction_id);
    }

    pub fn status_changed(env: &Env, transaction_id: u64, status: TransactionStatus) {
        env.events()
            .publish((symbol_short!("tx_stat"),), (transaction_id, status));
    }
}

/// Payment events
pub struct PaymentEvents;

impl PaymentEvents {
    pub fn created(env: &Env, payment_id: u64, recipient: Address, amount: i128) {
        env.events()
            .publish((symbol_short!("pay_new"),), (payment_id, recipient, amount));
    }

    pub fn processed(env: &Env, payment_id: u64) {
        env.events()
            .publish((symbol_short!("pay_proc"),), payment_id);
    }

    pub fn failed(env: &Env, payment_id: u64) {
        env.events()
            .publish((symbol_short!("pay_fail"),), payment_id);
    }
}

/// Reputation events
pub struct ReputationEvents;

impl ReputationEvents {
    pub fn score_updated(env: &Env, collector: Address, old_score: u32, new_score: u32) {
        env.events().publish(
            (symbol_short!("rep_upd"),),
            (collector, old_score, new_score),
        );
    }
}

/// Pricing events
pub struct PricingEvents;

impl PricingEvents {
    pub fn price_updated(env: &Env, material_type: MaterialType, price: i128) {
        env.events()
            .publish((symbol_short!("prc_upd"),), (material_type, price));
    }
}

/// Admin events
pub struct AdminEvents;

impl AdminEvents {
    pub fn admin_changed(env: &Env, old_admin: Address, new_admin: Address) {
        env.events()
            .publish((symbol_short!("adm_chg"),), (old_admin, new_admin));
    }

    pub fn paused(env: &Env) {
        env.events().publish((symbol_short!("paused"),), ());
    }

    pub fn unpaused(env: &Env) {
        env.events().publish((symbol_short!("unpaused"),), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_emit_token_events() {
        let env = Env::default();
        let addr = Address::generate(&env);

        TokenEvents::mint(&env, addr.clone(), 1000);
        TokenEvents::burn(&env, addr.clone(), 500);
        TokenEvents::transfer(&env, addr.clone(), Address::generate(&env), 250);

        // Events emitted successfully (no panic)
    }

    #[test]
    fn test_emit_collector_events() {
        let env = Env::default();
        let addr = Address::generate(&env);
        let name = String::from_str(&env, "Test");

        CollectorEvents::registered(&env, addr.clone(), name.clone());
        CollectorEvents::status_updated(&env, addr, CollectorStatus::Active);
    }

    #[test]
    fn test_emit_transaction_events() {
        let env = Env::default();
        let addr = Address::generate(&env);

        TransactionEvents::recorded(&env, 1, addr, MaterialType::Plastic, 5000);
        TransactionEvents::verified(&env, 1);
        TransactionEvents::status_changed(&env, 1, TransactionStatus::Completed);
    }
}
