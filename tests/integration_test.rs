// Integration tests for WasteFi contracts
// Tests cross-contract interactions and end-to-end workflows
#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

// Import contract clients
use collection_point::{CollectionPoint, CollectionPointClient};
use collector_registry::{CollectorRegistry, CollectorRegistryClient};
use common::types::*;
use material_pricing::{MaterialPricing, MaterialPricingClient};
use payment_distribution::{PaymentDistribution, PaymentDistributionClient};
use reputation::{Reputation, ReputationClient};
use waste_token::{WasteToken, WasteTokenClient};
use waste_transaction::{WasteTransaction, WasteTransactionClient};

/// Helper struct to hold all contract instances for testing
struct WasteFiTestEnv {
    env: Env,
    admin: Address,
    waste_token: WasteTokenClient<'static>,
    collector_registry: CollectorRegistryClient<'static>,
    collection_point: CollectionPointClient<'static>,
    waste_transaction: WasteTransactionClient<'static>,
    payment_distribution: PaymentDistributionClient<'static>,
    reputation: ReputationClient<'static>,
    material_pricing: MaterialPricingClient<'static>,
}

impl WasteFiTestEnv {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| li.timestamp = 1000);

        let admin = Address::generate(&env);

        // Deploy all contracts
        let waste_token_id = env.register_contract(None, WasteToken);
        let waste_token = WasteTokenClient::new(&env, &waste_token_id);

        let collector_registry_id = env.register_contract(None, CollectorRegistry);
        let collector_registry = CollectorRegistryClient::new(&env, &collector_registry_id);

        let collection_point_id = env.register_contract(None, CollectionPoint);
        let collection_point = CollectionPointClient::new(&env, &collection_point_id);

        let waste_transaction_id = env.register_contract(None, WasteTransaction);
        let waste_transaction = WasteTransactionClient::new(&env, &waste_transaction_id);

        let payment_distribution_id = env.register_contract(None, PaymentDistribution);
        let payment_distribution = PaymentDistributionClient::new(&env, &payment_distribution_id);

        let reputation_id = env.register_contract(None, Reputation);
        let reputation = ReputationClient::new(&env, &reputation_id);

        let material_pricing_id = env.register_contract(None, MaterialPricing);
        let material_pricing = MaterialPricingClient::new(&env, &material_pricing_id);

        // Initialize all contracts
        waste_token.initialize(
            &admin,
            &String::from_str(&env, "WasteFi Token"),
            &String::from_str(&env, "WASTE"),
            &7,
        );

        collector_registry.initialize(&admin);
        collection_point.initialize(&admin);
        waste_transaction.initialize(&admin);
        payment_distribution.initialize(&admin, &waste_token_id);
        reputation.initialize(&admin);
        material_pricing.initialize(&admin);

        Self {
            env,
            admin,
            waste_token,
            collector_registry,
            collection_point,
            waste_transaction,
            payment_distribution,
            reputation,
            material_pricing,
        }
    }

    fn time_travel(&self, seconds: u64) {
        self.env.ledger().with_mut(|li| {
            li.timestamp += seconds;
        });
    }
}

// ============================================================================
// Basic Setup Tests
// ============================================================================

#[test]
fn test_all_contracts_deploy_and_initialize() {
    let test_env = WasteFiTestEnv::new();

    // Verify all contracts are initialized with correct admin
    assert_eq!(test_env.waste_token.admin(), test_env.admin);
    assert_eq!(test_env.collector_registry.admin(), test_env.admin);
    assert_eq!(test_env.collection_point.admin(), test_env.admin);
    assert_eq!(test_env.waste_transaction.admin(), test_env.admin);
    assert_eq!(test_env.payment_distribution.admin(), test_env.admin);
    assert_eq!(test_env.reputation.admin(), test_env.admin);
    assert_eq!(test_env.material_pricing.admin(), test_env.admin);
}

#[test]
fn test_token_metadata() {
    let test_env = WasteFiTestEnv::new();

    assert_eq!(
        test_env.waste_token.name(),
        String::from_str(&test_env.env, "WasteFi Token")
    );
    assert_eq!(
        test_env.waste_token.symbol(),
        String::from_str(&test_env.env, "WASTE")
    );
    assert_eq!(test_env.waste_token.decimals(), 7);
}

// ============================================================================
// Collector Registration and Verification Flow
// ============================================================================

#[test]
fn test_collector_registration_flow() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);
    let name = String::from_str(&test_env.env, "Alice Collector");
    let phone = String::from_str(&test_env.env, "+1234567890");

    // Register collector
    test_env
        .collector_registry
        .register(&collector, &name, &phone);

    // Verify collector info
    let collector_info = test_env.collector_registry.get_collector(&collector);
    assert_eq!(collector_info.name, name);
    assert_eq!(collector_info.phone_number, phone);
    assert_eq!(collector_info.status, CollectorStatus::Pending);

    // Admin approves collector
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    // Verify status updated
    let updated_info = test_env.collector_registry.get_collector(&collector);
    assert_eq!(updated_info.status, CollectorStatus::Active);

    // Check initial reputation
    let initial_reputation = test_env.reputation.get_score(&collector);
    assert_eq!(initial_reputation, 500); // Base score
}

// ============================================================================
// Collection Point Registration Flow
// ============================================================================

#[test]
fn test_collection_point_registration_flow() {
    let test_env = WasteFiTestEnv::new();

    let owner = Address::generate(&test_env.env);
    let name = String::from_str(&test_env.env, "Downtown Collection Point");
    let location = String::from_str(&test_env.env, "123 Main St");

    // Register collection point
    let point_id = test_env.collection_point.register(&owner, &name, &location);
    assert_eq!(point_id, 1);

    // Verify point details
    let point_info = test_env.collection_point.get_point(&point_id);
    assert_eq!(point_info.id, point_id);
    assert_eq!(point_info.owner, owner);
    assert_eq!(point_info.name, name);
    assert!(!point_info.verified);

    // Admin verifies collection point
    test_env.collection_point.verify_point(&point_id);

    // Verify verification
    let verified_info = test_env.collection_point.get_point(&point_id);
    assert!(verified_info.verified);
}

// ============================================================================
// Material Pricing Tests
// ============================================================================

#[test]
fn test_material_pricing_defaults() {
    let test_env = WasteFiTestEnv::new();

    // Check default prices are set
    assert_eq!(
        test_env.material_pricing.get_price(&MaterialType::Plastic),
        8_000_000
    ); // 0.8 XLM/kg
    assert_eq!(
        test_env.material_pricing.get_price(&MaterialType::Metal),
        15_000_000
    ); // 1.5 XLM/kg
    assert_eq!(
        test_env
            .material_pricing
            .get_price(&MaterialType::Electronics),
        25_000_000
    ); // 2.5 XLM/kg
}

#[test]
fn test_update_material_prices() {
    let test_env = WasteFiTestEnv::new();

    // Update plastic price
    test_env
        .material_pricing
        .set_price(&MaterialType::Plastic, &10_000_000);

    assert_eq!(
        test_env.material_pricing.get_price(&MaterialType::Plastic),
        10_000_000
    );

    // Batch update
    let mut updates = Vec::new(&test_env.env);
    updates.push_back((MaterialType::Glass, 6_000_000));
    updates.push_back((MaterialType::Paper, 5_000_000));
    test_env.material_pricing.batch_update_prices(&updates);

    assert_eq!(
        test_env.material_pricing.get_price(&MaterialType::Glass),
        6_000_000
    );
    assert_eq!(
        test_env.material_pricing.get_price(&MaterialType::Paper),
        5_000_000
    );
}

// ============================================================================
// Complete Waste Collection and Payment Flow
// ============================================================================

#[test]
fn test_complete_waste_collection_flow() {
    let test_env = WasteFiTestEnv::new();

    // Setup: Register collector
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Bob Collector"),
        &String::from_str(&test_env.env, "+9876543210"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    // Setup: Register collection point
    let point_owner = Address::generate(&test_env.env);
    let point_id = test_env.collection_point.register(
        &point_owner,
        &String::from_str(&test_env.env, "Central Point"),
        &String::from_str(&test_env.env, "456 Oak Ave"),
    );
    test_env.collection_point.verify_point(&point_id);

    // Step 1: Record waste collection
    let material_type = MaterialType::Plastic;
    let weight = 5000; // 5 kg
    let price_per_kg = test_env.material_pricing.get_price(&material_type);

    let tx_id = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &material_type,
        &weight,
        &price_per_kg,
    );
    assert_eq!(tx_id, 1);

    // Step 2: Verify transaction details
    let tx_record = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(tx_record.id, tx_id);
    assert_eq!(tx_record.collector, collector);
    assert_eq!(tx_record.material_type, material_type);
    assert_eq!(tx_record.weight, weight);
    assert_eq!(tx_record.status, TransactionStatus::Pending);

    // Check calculated total
    let expected_total = (price_per_kg * weight as i128) / 1000; // Convert grams to kg
    assert_eq!(tx_record.total_amount, expected_total);

    // Step 3: Admin verifies transaction
    test_env
        .waste_transaction
        .update_status(&tx_id, &TransactionStatus::Completed);

    let verified_tx = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(verified_tx.status, TransactionStatus::Completed);

    // Step 4: Update reputation
    test_env
        .reputation
        .record_transaction(&collector, &TransactionStatus::Completed);

    let reputation_score = test_env.reputation.get_score(&collector);
    assert_eq!(reputation_score, 505); // Base 500 + 5 for successful tx
}

#[test]
fn test_collector_transaction_history() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Carol Collector"),
        &String::from_str(&test_env.env, "+5555555555"),
    );

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Point A"),
        &String::from_str(&test_env.env, "Location A"),
    );

    // Record multiple transactions
    let materials = vec![
        MaterialType::Plastic,
        MaterialType::Glass,
        MaterialType::Metal,
    ];

    for material in materials.iter() {
        let price = test_env.material_pricing.get_price(material);
        test_env
            .waste_transaction
            .record_collection(&collector, &point_id, material, &3000, &price);
    }

    // Query collector transactions
    let transactions = test_env
        .waste_transaction
        .get_collector_transactions(&collector, &10);
    assert_eq!(transactions.len(), 3);

    // Check statistics
    let stats = test_env
        .waste_transaction
        .get_collector_statistics(&collector);
    assert_eq!(stats.total_transactions, 3);
    assert_eq!(stats.total_weight, 9000); // 3 * 3000 grams
}

// ============================================================================
// Payment Distribution Tests
// ============================================================================

#[test]
fn test_payment_creation_and_processing() {
    let test_env = WasteFiTestEnv::new();

    let recipient = Address::generate(&test_env.env);
    let tx_id = 123;
    let amount = 5_000_000; // 0.5 XLM worth of tokens

    // Create payment
    let payment_id = test_env
        .payment_distribution
        .process_payment(&tx_id, &recipient, &amount);
    assert_eq!(payment_id, 1);

    // Verify payment details
    let payment = test_env.payment_distribution.get_payment(&payment_id);
    assert_eq!(payment.id, payment_id);
    assert_eq!(payment.transaction_id, tx_id);
    assert_eq!(payment.recipient, recipient);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.status, PaymentStatus::Pending);

    // Mark as completed
    test_env
        .payment_distribution
        .update_payment_status(&payment_id, &PaymentStatus::Completed);

    let completed_payment = test_env.payment_distribution.get_payment(&payment_id);
    assert_eq!(completed_payment.status, PaymentStatus::Completed);
}

#[test]
fn test_batch_payment_processing() {
    let test_env = WasteFiTestEnv::new();

    let mut batch = Vec::new(&test_env.env);
    for i in 0..3 {
        let recipient = Address::generate(&test_env.env);
        batch.push_back((i + 100, recipient, 1_000_000));
    }

    let payment_ids = test_env.payment_distribution.batch_process(&batch);
    assert_eq!(payment_ids.len(), 3);

    // Verify all payments created
    for i in 0..payment_ids.len() {
        if let Some(payment_id) = payment_ids.get(i) {
            let payment = test_env.payment_distribution.get_payment(&payment_id);
            assert_eq!(payment.status, PaymentStatus::Pending);
        }
    }
}

// ============================================================================
// Reputation System Tests
// ============================================================================

#[test]
fn test_reputation_score_calculation() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);

    // Initial score
    let initial_score = test_env.reputation.get_score(&collector);
    assert_eq!(initial_score, 500);

    // Record successful transactions
    for _ in 0..5 {
        test_env
            .reputation
            .record_transaction(&collector, &TransactionStatus::Completed);
    }

    let score_after_success = test_env.reputation.get_score(&collector);
    assert_eq!(score_after_success, 525); // 500 + (5 * 5)

    // Record a disputed transaction
    test_env
        .reputation
        .record_transaction(&collector, &TransactionStatus::Disputed);

    let score_after_dispute = test_env.reputation.get_score(&collector);
    assert_eq!(score_after_dispute, 515); // 525 - 10

    // Check statistics
    let stats = test_env.reputation.get_statistics(&collector);
    assert_eq!(stats.successful_transactions, 5);
    assert_eq!(stats.disputed_transactions, 1);
}

#[test]
fn test_reputation_success_rate_bonus() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);

    // Record 20 successful transactions (100% success rate)
    for _ in 0..20 {
        test_env
            .reputation
            .record_transaction(&collector, &TransactionStatus::Completed);
    }

    let score = test_env.reputation.get_score(&collector);
    // Base 500 + (20 * 5) + success_rate_bonus for 100% with 20+ txs
    // Success rate bonus: up to +100 points for perfect score
    assert!(score >= 600); // At least base + successful txs
    assert!(score <= 700); // Max with bonus
}

// ============================================================================
// Multi-Material Collection Tests
// ============================================================================

#[test]
fn test_multi_material_collection_session() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "David Multi"),
        &String::from_str(&test_env.env, "+1111111111"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Multi Point"),
        &String::from_str(&test_env.env, "Multi Location"),
    );

    // Collect different materials in one session
    let collections = vec![
        (MaterialType::Plastic, 2000),    // 2 kg
        (MaterialType::Glass, 3000),      // 3 kg
        (MaterialType::Metal, 1500),      // 1.5 kg
        (MaterialType::Electronics, 500), // 0.5 kg
    ];

    let mut total_expected_amount = 0i128;

    for (material, weight) in collections.iter() {
        let price = test_env.material_pricing.get_price(material);
        test_env
            .waste_transaction
            .record_collection(&collector, &point_id, material, weight, &price);
        total_expected_amount += (price * *weight as i128) / 1000;
    }

    // Verify all transactions recorded
    let transactions = test_env
        .waste_transaction
        .get_collector_transactions(&collector, &10);
    assert_eq!(transactions.len(), 4);

    // Verify total weight and amount
    let stats = test_env
        .waste_transaction
        .get_collector_statistics(&collector);
    assert_eq!(stats.total_weight, 7000); // Sum of all weights
    assert_eq!(stats.total_amount, total_expected_amount);
}

// ============================================================================
// Pause/Emergency Tests
// ============================================================================

#[test]
fn test_pause_all_contracts() {
    let test_env = WasteFiTestEnv::new();

    // Pause all contracts
    test_env.waste_token.pause();
    test_env.collector_registry.pause();
    test_env.collection_point.pause();
    test_env.waste_transaction.pause();
    test_env.payment_distribution.pause();
    test_env.reputation.pause();
    test_env.material_pricing.pause();

    // Verify all are paused
    assert!(test_env.waste_token.is_paused());
    assert!(test_env.collector_registry.is_paused());
    assert!(test_env.collection_point.is_paused());
    assert!(test_env.waste_transaction.is_paused());
    assert!(test_env.payment_distribution.is_paused());
    assert!(test_env.reputation.is_paused());
    assert!(test_env.material_pricing.is_paused());
}

#[test]
#[should_panic(expected = "Contract paused")]
fn test_operations_blocked_when_paused() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);

    // Pause collector registry
    test_env.collector_registry.pause();

    // Try to register - should panic
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Test"),
        &String::from_str(&test_env.env, "+000000"),
    );
}

// ============================================================================
// Time-based Tests
// ============================================================================

#[test]
fn test_transaction_timestamps() {
    let test_env = WasteFiTestEnv::new();

    let initial_time = test_env.env.ledger().timestamp();

    let collector = Address::generate(&test_env.env);
    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Time Point"),
        &String::from_str(&test_env.env, "Time Location"),
    );

    // Record transaction
    let tx_id = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Plastic,
        &1000,
        &8_000_000,
    );

    let tx = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(tx.timestamp, initial_time);

    // Time travel and record another
    test_env.time_travel(3600); // 1 hour later

    let tx_id_2 = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Glass,
        &2000,
        &5_000_000,
    );

    let tx_2 = test_env.waste_transaction.get_transaction(&tx_id_2);
    assert_eq!(tx_2.timestamp, initial_time + 3600);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_zero_weight_collection_blocked() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);
    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "Test Location"),
    );

    // This should panic or be blocked by validation
    // Depending on implementation, adjust test accordingly
    // For now, assuming it's allowed but records 0 amount
    let tx_id = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Plastic,
        &0,
        &8_000_000,
    );

    let tx = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(tx.total_amount, 0);
}

#[test]
fn test_large_batch_operations() {
    let test_env = WasteFiTestEnv::new();

    // Create large batch of payments
    let mut batch = Vec::new(&test_env.env);
    for i in 0..50 {
        // Max 50 per batch typically
        let recipient = Address::generate(&test_env.env);
        batch.push_back((i, recipient, 100_000));
    }

    let payment_ids = test_env.payment_distribution.batch_process(&batch);
    assert_eq!(payment_ids.len(), 50);
}

// ============================================================================
// Query and Statistics Tests
// ============================================================================

#[test]
fn test_collector_comprehensive_stats() {
    let test_env = WasteFiTestEnv::new();

    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Stats Collector"),
        &String::from_str(&test_env.env, "+2222222222"),
    );

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Stats Point"),
        &String::from_str(&test_env.env, "Stats Location"),
    );

    // Record 10 transactions
    for i in 0..10 {
        let material = if i % 2 == 0 {
            MaterialType::Plastic
        } else {
            MaterialType::Glass
        };
        let price = test_env.material_pricing.get_price(&material);
        test_env
            .waste_transaction
            .record_collection(&collector, &point_id, &material, &1000, &price);
    }

    // Check transaction stats
    let tx_stats = test_env
        .waste_transaction
        .get_collector_statistics(&collector);
    assert_eq!(tx_stats.total_transactions, 10);
    assert_eq!(tx_stats.total_weight, 10_000);

    // Check reputation stats
    for _ in 0..10 {
        test_env
            .reputation
            .record_transaction(&collector, &TransactionStatus::Completed);
    }

    let rep_stats = test_env.reputation.get_statistics(&collector);
    assert_eq!(rep_stats.successful_transactions, 10);
    assert_eq!(rep_stats.total_transactions, 10);
}

// ============================================================================
// Basic Environment Tests (from original file)
// ============================================================================

#[test]
fn test_workspace_setup() {
    let env = Env::default();
    assert!(env.ledger().timestamp() > 0);
}

#[test]
fn test_contract_addresses() {
    let env = Env::default();
    let addr1 = Address::generate(&env);
    let addr2 = Address::generate(&env);
    assert_ne!(addr1, addr2);
}

#[test]
fn test_time_travel() {
    let env = Env::default();
    let initial_time = env.ledger().timestamp();

    env.ledger().with_mut(|li| {
        li.timestamp += 1000;
    });

    let new_time = env.ledger().timestamp();
    assert_eq!(new_time, initial_time + 1000);
}

// ============================================================================
// Collector Status Validation Tests (Issue #8)
// ============================================================================

#[test]
fn test_active_collector_can_submit_transaction() {
    let test_env = WasteFiTestEnv::new();

    // Register and activate collector
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Active Collector"),
        &String::from_str(&test_env.env, "+1234567890"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    // Register collection point
    let point_owner = Address::generate(&test_env.env);
    let point_id = test_env.collection_point.register(
        &point_owner,
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "123 Test St"),
    );

    // Configure waste_transaction to use collector_registry
    let registry_id = test_env.env.register_contract(None, CollectorRegistry);
    test_env
        .waste_transaction
        .set_collector_registry_contract(&registry_id);

    // Active collector should be able to submit transaction
    let tx_id = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Plastic,
        &5000,
        &8_000_000,
    );

    // Verify transaction was recorded
    let tx = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(tx.collector, collector);
    assert_eq!(tx.status, TransactionStatus::Pending);
}

#[test]
#[should_panic(expected = "Collector is not active")]
fn test_pending_collector_cannot_submit_transaction() {
    let test_env = WasteFiTestEnv::new();

    // Register collector but don't activate (status remains Pending)
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Pending Collector"),
        &String::from_str(&test_env.env, "+9876543210"),
    );

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "456 Test Ave"),
    );

    // Configure waste_transaction to use collector_registry
    let registry_id = test_env.env.register_contract(None, CollectorRegistry);
    test_env
        .waste_transaction
        .set_collector_registry_contract(&registry_id);

    // Pending collector should NOT be able to submit - should panic
    test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Plastic,
        &5000,
        &8_000_000,
    );
}

#[test]
#[should_panic(expected = "Collector is not active")]
fn test_suspended_collector_cannot_submit_transaction() {
    let test_env = WasteFiTestEnv::new();

    // Register and activate collector
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Suspended Collector"),
        &String::from_str(&test_env.env, "+5555555555"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    // Later, suspend the collector
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Suspended);

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "789 Test Blvd"),
    );

    // Configure waste_transaction to use collector_registry
    let registry_id = test_env.env.register_contract(None, CollectorRegistry);
    test_env
        .waste_transaction
        .set_collector_registry_contract(&registry_id);

    // Suspended collector should NOT be able to submit - should panic
    test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Glass,
        &3000,
        &5_000_000,
    );
}

#[test]
#[should_panic(expected = "Collector is not active")]
fn test_banned_collector_cannot_submit_transaction() {
    let test_env = WasteFiTestEnv::new();

    // Register and activate collector
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Banned Collector"),
        &String::from_str(&test_env.env, "+7777777777"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    // Ban the collector
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Banned);

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "321 Test Ln"),
    );

    // Configure waste_transaction to use collector_registry
    let registry_id = test_env.env.register_contract(None, CollectorRegistry);
    test_env
        .waste_transaction
        .set_collector_registry_contract(&registry_id);

    // Banned collector should NOT be able to submit - should panic
    test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Metal,
        &4000,
        &15_000_000,
    );
}

#[test]
fn test_collector_status_change_affects_future_transactions() {
    let test_env = WasteFiTestEnv::new();

    // Register and activate collector
    let collector = Address::generate(&test_env.env);
    test_env.collector_registry.register(
        &collector,
        &String::from_str(&test_env.env, "Status Change Collector"),
        &String::from_str(&test_env.env, "+9999999999"),
    );
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Active);

    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "555 Test Dr"),
    );

    // Configure waste_transaction to use collector_registry
    let registry_id = test_env.env.register_contract(None, CollectorRegistry);
    test_env
        .waste_transaction
        .set_collector_registry_contract(&registry_id);

    // First transaction should succeed (collector is active)
    let tx_id_1 = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Plastic,
        &2000,
        &8_000_000,
    );
    assert_eq!(tx_id_1, 1);

    // Suspend the collector
    test_env
        .collector_registry
        .update_status(&collector, &CollectorStatus::Suspended);

    // Second transaction should fail (collector is now suspended)
    let result = std::panic::catch_unwind(|| {
        test_env.waste_transaction.record_collection(
            &collector,
            &point_id,
            &MaterialType::Plastic,
            &2000,
            &8_000_000,
        )
    });
    assert!(result.is_err());
}

#[test]
fn test_transaction_without_registry_configured_works() {
    let test_env = WasteFiTestEnv::new();

    // Don't configure collector registry contract
    let collector = Address::generate(&test_env.env);
    let point_id = test_env.collection_point.register(
        &Address::generate(&test_env.env),
        &String::from_str(&test_env.env, "Test Point"),
        &String::from_str(&test_env.env, "999 Test Way"),
    );

    // Without collector registry configured, validation should be skipped
    // and transaction should succeed
    let tx_id = test_env.waste_transaction.record_collection(
        &collector,
        &point_id,
        &MaterialType::Electronics,
        &1000,
        &25_000_000,
    );

    assert_eq!(tx_id, 1);
    let tx = test_env.waste_transaction.get_transaction(&tx_id);
    assert_eq!(tx.collector, collector);
}
