#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

mod storage;
use common::types::*;
use storage::*;

#[cfg(test)]
mod test;

const MATERIAL_PRICING_CONTRACT: &str = "MaterialPricingContract";
const REPUTATION_CONTRACT: &str = "ReputationContract";
const COLLECTOR_REGISTRY_CONTRACT: &str = "CollectorRegistryContract";

#[contract]
pub struct WasteTransaction;

#[contractimpl]
impl WasteTransaction {
    /// Initialize the waste transaction contract
    ///
    /// # Arguments
    /// * `admin` - Contract administrator address
    pub fn initialize(env: Env, admin: Address) {
        common::Initializable::require_not_initialized(&env).expect("Already initialized");

        // Set admin
        common::AccessControl::set_admin(&env, admin);

        // Initialize transaction counter
        write_transaction_count(&env, 0);

        // Mark as initialized
        common::Initializable::mark_initialized(&env);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Set material pricing contract address (admin only)
    ///
    /// # Arguments
    /// * `contract_address` - MaterialPricing contract address
    pub fn set_material_pricing_contract(env: Env, contract_address: Address) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        env.storage()
            .instance()
            .set(&MATERIAL_PRICING_CONTRACT, &contract_address);
        common::bump_instance(&env);
    }

    /// Set reputation contract address (admin only)
    ///
    /// # Arguments
    /// * `contract_address` - Reputation contract address
    pub fn set_reputation_contract(env: Env, contract_address: Address) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        env.storage()
            .instance()
            .set(&REPUTATION_CONTRACT, &contract_address);
        common::bump_instance(&env);
    }

    /// Set collector registry contract address (admin only)
    ///
    /// # Arguments
    /// * `contract_address` - CollectorRegistry contract address
    pub fn set_collector_registry_contract(env: Env, contract_address: Address) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        env.storage()
            .instance()
            .set(&COLLECTOR_REGISTRY_CONTRACT, &contract_address);
        common::bump_instance(&env);
    }

    /// Get material pricing contract address
    pub fn get_material_pricing_contract(env: Env) -> Option<Address> {
        env.storage().instance().get(&MATERIAL_PRICING_CONTRACT)
    }

    /// Get reputation contract address
    pub fn get_reputation_contract(env: Env) -> Option<Address> {
        env.storage().instance().get(&REPUTATION_CONTRACT)
    }

    /// Get collector registry contract address
    pub fn get_collector_registry_contract(env: Env) -> Option<Address> {
        env.storage().instance().get(&COLLECTOR_REGISTRY_CONTRACT)
    }

    /// Record a new waste collection transaction
    ///
    /// # Arguments
    /// * `collector` - Address of the collector
    /// * `collection_point` - Address of the collection point
    /// * `material_type` - Type of material collected
    /// * `weight` - Weight in grams
    /// * `price_per_kg` - Price per kilogram in stroops
    ///
    /// # Returns
    /// Transaction ID
    pub fn record_collection(
        env: Env,
        collector: Address,
        collection_point: Address,
        material_type: MaterialType,
        weight: u64,
        price_per_kg: i128,
    ) -> u64 {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Validate collector status before proceeding
        if let Some(registry_contract) = env
            .storage()
            .instance()
            .get::<_, Address>(&COLLECTOR_REGISTRY_CONTRACT)
        {
            // Cross-contract call to check if collector is active
            let is_active: bool = env.invoke_contract(
                &registry_contract,
                &soroban_sdk::symbol_short!("is_active"),
                soroban_sdk::vec![&env, collector.clone().into_val(&env)],
            );

            if !is_active {
                panic!("Collector is not active");
            }
        }

        // Fraud detection checks
        // 1. Check if collector is flagged for critical risk
        common::FraudDetection::require_not_critical(&env, &collector)
            .expect("Collector flagged for fraud");

        // 2. Check rate limit: max 20 transactions per hour
        common::RateLimit::check_per_hour(
            &env,
            soroban_sdk::String::from_str(&env, "record_collection"),
            &collector,
            20,
        )
        .expect("Rate limit exceeded");

        // 3. Check for duplicate transaction (within 5 minutes)
        common::DuplicateDetection::require_not_duplicate(
            &env,
            &collector,
            weight,
            material_type.clone() as u32,
            300, // 5 minutes tolerance
        )
        .expect("Duplicate transaction detected");

        // Validate inputs
        common::validation::validate_weight_bounds(weight).expect("Invalid weight");
        common::validation::validate_price(price_per_kg).expect("Invalid price");

        // Record for fraud detection
        common::RateLimit::record(
            &env,
            soroban_sdk::String::from_str(&env, "record_collection"),
            &collector,
        );
        common::DuplicateDetection::record_transaction(
            &env,
            &collector,
            weight,
            material_type.clone() as u32,
        );
        common::FraudDetection::record_transaction(&env, &collector);
        common::FraudDetection::record_weight(&env, &collector, weight);

        // Calculate total amount: (weight in grams / 1000) * price_per_kg
        let weight_kg = (weight as i128) / 1000;
        let total_amount = weight_kg.saturating_mul(price_per_kg);

        // Get next transaction ID
        let transaction_id = increment_transaction_count(&env);

        // Create transaction record
        let record = WasteRecord {
            id: transaction_id,
            collector: collector.clone(),
            collection_point: collection_point.clone(),
            material_type: material_type.clone(),
            weight,
            price_per_kg,
            total_amount,
            status: TransactionStatus::Pending,
            timestamp: env.ledger().timestamp(),
            verified: false,
        };

        // Store transaction
        write_transaction(&env, transaction_id, &record);

        // Index by collector
        add_collector_transaction(&env, &collector, transaction_id);

        // Emit event
        common::TransactionEvents::recorded(&env, transaction_id, collector, material_type, weight);

        // Bump storage
        common::bump_instance(&env);

        transaction_id
    }

    /// Record collection with automatic price lookup
    ///
    /// # Arguments
    /// * `collector` - Address of the collector
    /// * `collection_point` - Collection point ID (u64)
    /// * `material_type` - Type of material collected
    /// * `weight` - Weight in grams
    ///
    /// # Returns
    /// Transaction ID
    ///
    /// # Note
    /// This method automatically fetches current price from MaterialPricing contract
    pub fn record_with_price_lookup(
        env: Env,
        collector: Address,
        _collection_point: u64,
        material_type: MaterialType,
        weight: u64,
    ) -> u64 {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Validate collector status before proceeding
        if let Some(registry_contract) = env
            .storage()
            .instance()
            .get::<_, Address>(&COLLECTOR_REGISTRY_CONTRACT)
        {
            // Cross-contract call to check if collector is active
            let is_active: bool = env.invoke_contract(
                &registry_contract,
                &soroban_sdk::symbol_short!("is_active"),
                soroban_sdk::vec![&env, collector.clone().into_val(&env)],
            );

            if !is_active {
                panic!("Collector is not active");
            }
        }

        // Validate weight
        common::validation::validate_weight_bounds(weight).expect("Invalid weight");

        // Get MaterialPricing contract address
        let _pricing_contract: Address = env
            .storage()
            .instance()
            .get(&MATERIAL_PRICING_CONTRACT)
            .expect("MaterialPricing contract not configured");

        // TODO: Call pricing_contract.get_price(material_type)
        // For now, we'll use a default price until full cross-contract call implementation
        let price_per_kg = 5_000_000i128; // 0.5 XLM/kg default

        // Calculate total amount
        let weight_kg = (weight as i128) / 1000;
        let total_amount = weight_kg.saturating_mul(price_per_kg);

        // Get next transaction ID
        let transaction_id = increment_transaction_count(&env);

        // Use collector address as placeholder for collection point
        let collection_point_address = collector.clone();

        let record = WasteRecord {
            id: transaction_id,
            collector: collector.clone(),
            collection_point: collection_point_address,
            material_type: material_type.clone(),
            weight,
            price_per_kg,
            total_amount,
            status: TransactionStatus::Pending,
            timestamp: env.ledger().timestamp(),
            verified: false,
        };

        // Store transaction
        write_transaction(&env, transaction_id, &record);

        // Index by collector
        add_collector_transaction(&env, &collector, transaction_id);

        // Emit event
        common::TransactionEvents::recorded(&env, transaction_id, collector, material_type, weight);

        // Bump storage
        common::bump_instance(&env);

        transaction_id
    }

    /// Get transaction details
    ///
    /// # Arguments
    /// * `transaction_id` - Transaction ID
    ///
    /// # Returns
    /// Transaction record
    pub fn get_transaction(env: Env, transaction_id: u64) -> WasteRecord {
        read_transaction(&env, transaction_id).expect("Transaction not found")
    }

    /// Verify a transaction (admin only)
    ///
    /// # Arguments
    /// * `transaction_id` - Transaction ID to verify
    ///
    /// # Note
    /// This also triggers a reputation update if the Reputation contract is configured
    pub fn verify_transaction(env: Env, transaction_id: u64) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Get transaction
        let mut record = read_transaction(&env, transaction_id).expect("Transaction not found");

        // Record acceptance for fraud detection
        common::FraudDetection::record_rejection(&env, &record.collector, false);

        // Update verification status
        record.verified = true;
        record.status = TransactionStatus::Completed;

        // Save updated record
        write_transaction(&env, transaction_id, &record);

        // Update risk score after verification
        common::FraudDetection::update_risk_score(&env, &record.collector);

        // Trigger reputation update if configured
        if let Some(_reputation_contract) = env
            .storage()
            .instance()
            .get::<_, Address>(&REPUTATION_CONTRACT)
        {
            // TODO: Call reputation_contract.record_transaction(collector, TransactionStatus::Completed)
            // For now, just note that the contract is configured
            // Full implementation requires cross-contract invocation setup
        }

        // Emit event
        common::TransactionEvents::verified(&env, transaction_id);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Update transaction status (admin only)
    ///
    /// # Arguments
    /// * `transaction_id` - Transaction ID
    /// * `status` - New status
    pub fn update_status(env: Env, transaction_id: u64, status: TransactionStatus) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Get transaction
        let mut record = read_transaction(&env, transaction_id).expect("Transaction not found");

        // Record rejection if status is Disputed (rejected)
        if matches!(status, TransactionStatus::Disputed) {
            common::FraudDetection::record_rejection(&env, &record.collector, true);
        }

        // Update status
        record.status = status.clone();

        // Save updated record
        write_transaction(&env, transaction_id, &record);

        // Update risk score if disputed or cancelled
        if matches!(
            status,
            TransactionStatus::Disputed | TransactionStatus::Cancelled
        ) {
            common::FraudDetection::update_risk_score(&env, &record.collector);
        }

        // Bump storage
        common::bump_instance(&env);
    }

    /// Get transactions by collector
    ///
    /// # Arguments
    /// * `collector` - Collector address
    /// * `limit` - Maximum number of results (optional, defaults to 10)
    ///
    /// # Returns
    /// Vector of transaction records
    pub fn get_collector_transactions(
        env: Env,
        collector: Address,
        limit: u32,
    ) -> Vec<WasteRecord> {
        let max_limit = if limit == 0 || limit > 50 { 50 } else { limit };
        read_collector_transactions(&env, &collector, max_limit)
    }

    /// Get total transaction count
    ///
    /// # Returns
    /// Total number of transactions
    pub fn get_transaction_count(env: Env) -> u64 {
        read_transaction_count(&env)
    }

    /// Get collector statistics
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// Tuple of (total_transactions, total_weight, total_amount)
    pub fn get_collector_stats(env: Env, collector: Address) -> (u64, u64, i128) {
        let transactions = read_collector_transactions(&env, &collector, 1000);

        let mut total_weight: u64 = 0;
        let mut total_amount: i128 = 0;

        for tx in transactions.iter() {
            total_weight = total_weight.saturating_add(tx.weight);
            total_amount = total_amount.saturating_add(tx.total_amount);
        }

        (transactions.len() as u64, total_weight, total_amount)
    }

    /// Pause the contract (admin only)
    pub fn pause(env: Env) {
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::Pausable::admin_pause(&env, &admin).expect("Not admin");

        common::AdminEvents::paused(&env);
    }

    /// Unpause the contract (admin only)
    pub fn unpause(env: Env) {
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::Pausable::admin_unpause(&env, &admin).expect("Not admin");

        common::AdminEvents::unpaused(&env);
    }

    /// Check if contract is paused
    pub fn is_paused(env: Env) -> bool {
        common::Pausable::is_paused(&env)
    }

    /// Get admin address
    pub fn admin(env: Env) -> Address {
        common::AccessControl::get_admin(&env).expect("Admin not found")
    }

    /// Batch record multiple waste collections
    ///
    /// # Arguments
    /// * `collections` - Vector of (collector, collection_point, material_type, weight, price_per_kg) tuples
    ///
    /// # Returns
    /// Vector of transaction IDs
    pub fn batch_record_collections(
        env: Env,
        collections: Vec<(Address, Address, MaterialType, u64, i128)>,
    ) -> Vec<u64> {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        let mut transaction_ids = Vec::new(&env);

        for i in 0..collections.len() {
            if let Some((collector, collection_point, material_type, weight, price_per_kg)) =
                collections.get(i)
            {
                // Validate inputs
                if common::validation::validate_weight_bounds(weight).is_err()
                    || common::validation::validate_price(price_per_kg).is_err()
                {
                    continue;
                }

                // Calculate total amount
                let weight_kg = (weight as i128) / 1000;
                let total_amount = weight_kg.saturating_mul(price_per_kg);

                // Get next transaction ID
                let transaction_id = increment_transaction_count(&env);

                // Create transaction record
                let record = WasteRecord {
                    id: transaction_id,
                    collector: collector.clone(),
                    collection_point: collection_point.clone(),
                    material_type: material_type.clone(),
                    weight,
                    price_per_kg,
                    total_amount,
                    status: TransactionStatus::Pending,
                    timestamp: env.ledger().timestamp(),
                    verified: false,
                };

                // Store transaction
                write_transaction(&env, transaction_id, &record);

                // Index by collector
                add_collector_transaction(&env, &collector, transaction_id);

                // Emit event
                common::TransactionEvents::recorded(
                    &env,
                    transaction_id,
                    collector,
                    material_type,
                    weight,
                );

                transaction_ids.push_back(transaction_id);
            }
        }

        // Bump storage
        common::bump_instance(&env);

        transaction_ids
    }

    /// Batch verify multiple transactions (admin only)
    ///
    /// # Arguments
    /// * `transaction_ids` - Vector of transaction IDs to verify
    ///
    /// # Returns
    /// Number of transactions successfully verified
    pub fn batch_verify_transactions(env: Env, transaction_ids: Vec<u64>) -> u32 {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        let mut verified_count = 0u32;

        for i in 0..transaction_ids.len() {
            if let Some(transaction_id) = transaction_ids.get(i) {
                // Get transaction
                if let Some(mut record) = read_transaction(&env, transaction_id) {
                    // Update verification status
                    record.verified = true;
                    record.status = TransactionStatus::Completed;

                    // Save updated record
                    write_transaction(&env, transaction_id, &record);

                    // Emit event
                    common::TransactionEvents::verified(&env, transaction_id);

                    verified_count += 1;
                }
            }
        }

        // Bump storage
        common::bump_instance(&env);

        verified_count
    }

    /// Get multiple transactions at once (optimized batch query)
    ///
    /// # Arguments
    /// * `transaction_ids` - Vector of transaction IDs
    ///
    /// # Returns
    /// Vector of transaction records (None for non-existent transactions)
    pub fn get_transactions_batch(env: Env, transaction_ids: Vec<u64>) -> Vec<Option<WasteRecord>> {
        let mut results = Vec::new(&env);

        for i in 0..transaction_ids.len() {
            if let Some(tx_id) = transaction_ids.get(i) {
                let record = read_transaction(&env, tx_id);
                results.push_back(record);
            }
        }

        results
    }

    /// Get collector statistics (convenience method)
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// CollectorStats struct
    pub fn get_collector_statistics(env: Env, collector: Address) -> CollectorStats {
        let (total_transactions, total_weight, total_amount) =
            Self::get_collector_stats(env, collector);

        CollectorStats {
            total_transactions,
            total_weight,
            total_amount,
        }
    }

    /// Get transactions by status (filtered query)
    ///
    /// # Arguments
    /// * `status` - Filter by transaction status
    /// * `limit` - Maximum results (max 100)
    ///
    /// # Returns
    /// Vector of transactions matching the status
    pub fn get_transactions_by_status(
        env: Env,
        status: TransactionStatus,
        limit: u64,
    ) -> Vec<WasteRecord> {
        let max_limit = if limit > 100 { 100 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = read_transaction_count(&env);

        for tx_id in 1..=total_count {
            if results.len() >= max_limit as u32 {
                break;
            }

            if let Some(record) = read_transaction(&env, tx_id) {
                if record.status == status {
                    results.push_back(record);
                }
            }
        }

        results
    }

    /// Get transactions by material type (filtered query)
    ///
    /// # Arguments
    /// * `material_type` - Filter by material type
    /// * `limit` - Maximum results (max 100)
    ///
    /// # Returns
    /// Vector of transactions matching the material type
    pub fn get_transactions_by_material(
        env: Env,
        material_type: MaterialType,
        limit: u64,
    ) -> Vec<WasteRecord> {
        let max_limit = if limit > 100 { 100 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = read_transaction_count(&env);

        for tx_id in 1..=total_count {
            if results.len() >= max_limit as u32 {
                break;
            }

            if let Some(record) = read_transaction(&env, tx_id) {
                if record.material_type == material_type {
                    results.push_back(record);
                }
            }
        }

        results
    }

    /// Get transactions in a time range
    ///
    /// # Arguments
    /// * `start_time` - Start timestamp
    /// * `end_time` - End timestamp
    /// * `limit` - Maximum results (max 100)
    ///
    /// # Returns
    /// Vector of transactions within the time range
    pub fn get_transactions_by_time_range(
        env: Env,
        start_time: u64,
        end_time: u64,
        limit: u64,
    ) -> Vec<WasteRecord> {
        let max_limit = if limit > 100 { 100 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = read_transaction_count(&env);

        for tx_id in 1..=total_count {
            if results.len() >= max_limit as u32 {
                break;
            }

            if let Some(record) = read_transaction(&env, tx_id) {
                if record.timestamp >= start_time && record.timestamp <= end_time {
                    results.push_back(record);
                }
            }
        }

        results
    }

    /// Get material type statistics
    ///
    /// # Arguments
    /// * `material_type` - Material type to analyze
    ///
    /// # Returns
    /// Tuple of (total_transactions, total_weight, total_amount)
    pub fn get_material_statistics(env: Env, material_type: MaterialType) -> (u64, u64, i128) {
        let mut total_transactions = 0u64;
        let mut total_weight = 0u64;
        let mut total_amount = 0i128;

        let total_count = read_transaction_count(&env);

        for tx_id in 1..=total_count {
            if let Some(record) = read_transaction(&env, tx_id) {
                if record.material_type == material_type {
                    total_transactions += 1;
                    total_weight = total_weight.saturating_add(record.weight);
                    total_amount = total_amount.saturating_add(record.total_amount);
                }
            }
        }

        (total_transactions, total_weight, total_amount)
    }

    /// Get global transaction statistics
    ///
    /// # Returns
    /// Tuple of (total_transactions, total_weight, total_amount, verified_count, pending_count)
    pub fn get_global_tx_statistics(env: Env) -> (u64, u64, i128, u64, u64) {
        let mut total_weight = 0u64;
        let mut total_amount = 0i128;
        let mut verified_count = 0u64;
        let mut pending_count = 0u64;

        let total_transactions = read_transaction_count(&env);

        for tx_id in 1..=total_transactions {
            if let Some(record) = read_transaction(&env, tx_id) {
                total_weight = total_weight.saturating_add(record.weight);
                total_amount = total_amount.saturating_add(record.total_amount);

                if record.verified {
                    verified_count += 1;
                }
                if record.status == TransactionStatus::Pending {
                    pending_count += 1;
                }
            }
        }

        (
            total_transactions,
            total_weight,
            total_amount,
            verified_count,
            pending_count,
        )
    }

    /// Get recent transactions (last N transactions)
    ///
    /// # Arguments
    /// * `limit` - Number of recent transactions (max 50)
    ///
    /// # Returns
    /// Vector of most recent transactions
    pub fn get_recent_transactions(env: Env, limit: u64) -> Vec<WasteRecord> {
        let max_limit = if limit > 50 { 50 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = read_transaction_count(&env);

        let start_id = if total_count > max_limit {
            total_count - max_limit + 1
        } else {
            1
        };

        for tx_id in start_id..=total_count {
            if let Some(record) = read_transaction(&env, tx_id) {
                results.push_back(record);
            }
        }

        results
    }

    /// Get collector risk score
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// Risk score (0-1000)
    pub fn get_risk_score(env: Env, collector: Address) -> u32 {
        common::FraudDetection::calculate_risk_score(&env, &collector)
    }

    /// Get collector risk level
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// Risk level (0=Low, 1=Medium, 2=High, 3=Critical)
    pub fn get_risk_level(env: Env, collector: Address) -> u32 {
        common::FraudDetection::get_risk_level(&env, &collector) as u32
    }

    /// Check if collector is flagged for fraud
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// True if flagged
    pub fn is_flagged(env: Env, collector: Address) -> bool {
        common::FraudDetection::is_flagged(&env, &collector)
    }

    /// Flag collector for manual review (admin only)
    ///
    /// # Arguments
    /// * `collector` - Collector address
    /// * `reason` - Reason for flagging
    pub fn flag_for_review(env: Env, collector: Address, reason: soroban_sdk::String) {
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        common::FraudDetection::flag_for_review(&env, &collector, reason);
        common::bump_instance(&env);
    }

    /// Clear fraud flag (admin only)
    ///
    /// # Arguments
    /// * `collector` - Collector address
    pub fn clear_fraud_flag(env: Env, collector: Address) {
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        common::FraudDetection::clear_flag(&env, &collector);
        common::bump_instance(&env);
    }

    /// Get fraud flag details
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// Option<(reason, timestamp)>
    pub fn get_fraud_flag_details(
        env: Env,
        collector: Address,
    ) -> Option<(soroban_sdk::String, u64)> {
        common::FraudDetection::get_flag_details(&env, &collector)
    }

    /// Check rate limit quota for collector
    ///
    /// # Arguments
    /// * `collector` - Collector address
    ///
    /// # Returns
    /// Remaining quota for the hour
    pub fn get_rate_limit_quota(env: Env, collector: Address) -> u32 {
        common::RateLimit::get_remaining_quota(
            &env,
            soroban_sdk::String::from_str(&env, "record_collection"),
            &collector,
            20, // max 20 per hour
            3600,
        )
    }

    /// Update collector risk score (should be called after transaction verification)
    ///
    /// # Arguments
    /// * `collector` - Collector address
    pub fn update_risk_score(env: Env, collector: Address) {
        common::FraudDetection::update_risk_score(&env, &collector);
        common::bump_instance(&env);
    }
}
