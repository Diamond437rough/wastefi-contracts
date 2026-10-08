#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

use common::types::*;

#[cfg(test)]
mod test;

const TOKEN_CONTRACT: &str = "TokenContract";
const PAYMENT_COUNT: &str = "PaymentCount";

#[contract]
pub struct PaymentDistribution;

#[contractimpl]
impl PaymentDistribution {
    /// Initialize the payment distribution contract
    ///
    /// # Arguments
    /// * `admin` - Contract administrator address
    /// * `token_contract` - WasteToken contract address for minting rewards
    pub fn initialize(env: Env, admin: Address, token_contract: Address) {
        common::Initializable::require_not_initialized(&env).expect("Already initialized");

        // Set admin
        common::AccessControl::set_admin(&env, admin);

        // Store token contract address
        env.storage()
            .instance()
            .set(&TOKEN_CONTRACT, &token_contract);

        // Initialize payment counter
        env.storage().instance().set(&PAYMENT_COUNT, &0u64);

        // Mark as initialized
        common::Initializable::mark_initialized(&env);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Process payment for a completed transaction
    ///
    /// # Arguments
    /// * `transaction_id` - Transaction ID from WasteTransaction contract
    /// * `recipient` - Payment recipient address
    /// * `amount` - Payment amount in stroops
    ///
    /// # Returns
    /// Payment ID
    ///
    /// # Note
    /// This is a simplified version. Full implementation would call WasteTransaction
    /// contract to fetch transaction details and validate completion status.
    pub fn process_payment(env: Env, transaction_id: u64, recipient: Address, amount: i128) -> u64 {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Validate amount is positive
        if amount <= 0 {
            panic!("Amount must be positive");
        }

        // Get next payment ID
        let payment_count: u64 = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0);
        let payment_id = payment_count + 1;
        env.storage().instance().set(&PAYMENT_COUNT, &payment_id);

        // Create payment record
        let payment = Payment {
            id: payment_id,
            recipient: recipient.clone(),
            amount,
            status: PaymentStatus::Pending,
            transaction_id,
            created_at: env.ledger().timestamp(),
            processed_at: 0,
        };

        // Store payment
        let key = common::StorageKey::Payment(payment_id);
        env.storage().persistent().set(&key, &payment);
        common::bump_persistent(&env, &key);

        // Index by recipient
        let recipient_key = ("PaymentsByRecipient", recipient.clone());
        let mut recipient_payments: Vec<u64> = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or(Vec::new(&env));
        recipient_payments.push_back(payment_id);
        env.storage()
            .persistent()
            .set(&recipient_key, &recipient_payments);

        // Emit event
        common::PaymentEvents::created(&env, payment_id, recipient, amount);

        // Bump storage
        common::bump_instance(&env);

        payment_id
    }

    /// Get payment details
    ///
    /// # Arguments
    /// * `payment_id` - Payment ID
    ///
    /// # Returns
    /// Payment record
    pub fn get_payment(env: Env, payment_id: u64) -> Payment {
        let key = common::StorageKey::Payment(payment_id);
        env.storage()
            .persistent()
            .get(&key)
            .expect("Payment not found")
    }

    /// Get payments by recipient
    ///
    /// # Arguments
    /// * `recipient` - Recipient address
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Vector of payment records
    pub fn get_recipient_payments(env: Env, recipient: Address, limit: u32) -> Vec<Payment> {
        let max_limit = if limit == 0 || limit > 50 { 50 } else { limit };

        let recipient_key = ("PaymentsByRecipient", recipient);
        let payment_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or(Vec::new(&env));

        let mut payments = Vec::new(&env);

        // Get the most recent payments up to limit
        let start_idx = if payment_ids.len() > max_limit {
            payment_ids.len() - max_limit
        } else {
            0
        };

        for i in start_idx..payment_ids.len() {
            if let Some(payment_id) = payment_ids.get(i) {
                let key = common::StorageKey::Payment(payment_id);
                if let Some(payment) = env.storage().persistent().get(&key) {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    /// Batch process payments for multiple transactions
    ///
    /// # Arguments
    /// * `payments` - Vector of (transaction_id, recipient, amount) tuples
    ///
    /// # Returns
    /// Vector of payment IDs
    ///
    /// # Optimizations
    /// - Single admin and initialization check for the batch
    /// - Batch storage operations to minimize redundant reads/writes
    /// - Single payment counter update for all payments
    pub fn batch_process(env: Env, payments: Vec<(u64, Address, i128)>) -> Vec<u64> {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Single admin check for entire batch (optimization)
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Get current ledger timestamp once
        let timestamp = env.ledger().timestamp();

        // Get starting payment count once (optimization)
        let mut payment_count: u64 = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0);

        let mut payment_ids = Vec::new(&env);

        // Process all payments with batched storage operations
        for i in 0..payments.len() {
            if let Some((tx_id, recipient, amount)) = payments.get(i) {
                // Validate amount
                if amount <= 0 {
                    panic!("Amount must be positive");
                }

                // Generate next payment ID
                payment_count += 1;
                let payment_id = payment_count;

                // Create payment record
                let payment = Payment {
                    id: payment_id,
                    recipient: recipient.clone(),
                    amount,
                    status: PaymentStatus::Pending,
                    transaction_id: tx_id,
                    created_at: timestamp,
                    processed_at: 0,
                };

                // Store payment
                let key = common::StorageKey::Payment(payment_id);
                env.storage().persistent().set(&key, &payment);
                common::bump_persistent(&env, &key);

                // Index by recipient
                let recipient_key = ("PaymentsByRecipient", recipient.clone());
                let mut recipient_payments: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&recipient_key)
                    .unwrap_or(Vec::new(&env));
                recipient_payments.push_back(payment_id);
                env.storage()
                    .persistent()
                    .set(&recipient_key, &recipient_payments);

                // Emit event
                common::PaymentEvents::created(&env, payment_id, recipient, amount);

                payment_ids.push_back(payment_id);
            }
        }

        // Single update of payment counter (optimization)
        env.storage().instance().set(&PAYMENT_COUNT, &payment_count);

        // Single storage bump at the end (optimization)
        common::bump_instance(&env);

        payment_ids
    }

    /// Update payment status (admin only)
    ///
    /// # Arguments
    /// * `payment_id` - Payment ID
    /// * `status` - New status
    pub fn update_payment_status(env: Env, payment_id: u64, status: PaymentStatus) {
        common::Initializable::require_initialized(&env).expect("Not initialized");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Get payment
        let key = common::StorageKey::Payment(payment_id);
        let mut payment: Payment = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Payment not found");

        // Update status
        let status_clone = status.clone();
        payment.status = status;
        if status_clone == PaymentStatus::Completed {
            payment.processed_at = env.ledger().timestamp();
        }

        // Save updated payment
        env.storage().persistent().set(&key, &payment);
        common::bump_persistent(&env, &key);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Get total payment count
    ///
    /// # Returns
    /// Total number of payments
    pub fn get_payment_count(env: Env) -> u64 {
        env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0)
    }

    /// Get token contract address
    ///
    /// # Returns
    /// Token contract address
    pub fn get_token_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&TOKEN_CONTRACT)
            .expect("Token contract not set")
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

    /// Get payments by status (filtered query)
    ///
    /// # Arguments
    /// * `status` - Filter by payment status
    /// * `limit` - Maximum results (max 100)
    ///
    /// # Returns
    /// Vector of payments matching the status
    pub fn get_payments_by_status(env: Env, status: PaymentStatus, limit: u64) -> Vec<Payment> {
        let max_limit = if limit > 100 { 100 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0u64);

        for payment_id in 1..=total_count {
            if results.len() >= max_limit as u32 {
                break;
            }

            let key = common::StorageKey::Payment(payment_id);
            if let Some(payment) = env.storage().persistent().get::<_, Payment>(&key) {
                if payment.status == status {
                    results.push_back(payment);
                }
            }
        }

        results
    }

    /// Get recipient payment statistics
    ///
    /// # Arguments
    /// * `recipient` - Recipient address
    ///
    /// # Returns
    /// Tuple of (total_payments, total_amount, completed_count, pending_count)
    pub fn get_recipient_statistics(env: Env, recipient: Address) -> (u64, i128, u64, u64) {
        let recipient_key = ("PaymentsByRecipient", recipient);
        let payment_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or(Vec::new(&env));

        let mut total_amount = 0i128;
        let mut completed_count = 0u64;
        let mut pending_count = 0u64;

        for i in 0..payment_ids.len() {
            if let Some(payment_id) = payment_ids.get(i) {
                let key = common::StorageKey::Payment(payment_id);
                if let Some(payment) = env.storage().persistent().get::<_, Payment>(&key) {
                    total_amount = total_amount.saturating_add(payment.amount);

                    match payment.status {
                        PaymentStatus::Completed => completed_count += 1,
                        PaymentStatus::Pending => pending_count += 1,
                        _ => {}
                    }
                }
            }
        }

        let total_payments = payment_ids.len() as u64;
        (total_payments, total_amount, completed_count, pending_count)
    }

    /// Get global payment statistics
    ///
    /// # Returns
    /// Tuple of (total_payments, total_amount, completed_count, pending_count, failed_count)
    pub fn get_global_payment_statistics(env: Env) -> (u64, i128, u64, u64, u64) {
        let total_payments = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0u64);
        let mut total_amount = 0i128;
        let mut completed_count = 0u64;
        let mut pending_count = 0u64;
        let mut failed_count = 0u64;

        for payment_id in 1..=total_payments {
            let key = common::StorageKey::Payment(payment_id);
            if let Some(payment) = env.storage().persistent().get::<_, Payment>(&key) {
                total_amount = total_amount.saturating_add(payment.amount);

                match payment.status {
                    PaymentStatus::Completed => completed_count += 1,
                    PaymentStatus::Pending => pending_count += 1,
                    PaymentStatus::Failed => failed_count += 1,
                    _ => {}
                }
            }
        }

        (
            total_payments,
            total_amount,
            completed_count,
            pending_count,
            failed_count,
        )
    }

    /// Get recent payments (last N payments)
    ///
    /// # Arguments
    /// * `limit` - Number of recent payments (max 50)
    ///
    /// # Returns
    /// Vector of most recent payments
    pub fn get_recent_payments(env: Env, limit: u64) -> Vec<Payment> {
        let max_limit = if limit > 50 { 50 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0u64);

        let start_id = if total_count > max_limit {
            total_count - max_limit + 1
        } else {
            1
        };

        for payment_id in start_id..=total_count {
            let key = common::StorageKey::Payment(payment_id);
            if let Some(payment) = env.storage().persistent().get(&key) {
                results.push_back(payment);
            }
        }

        results
    }

    /// Get payments in a time range
    ///
    /// # Arguments
    /// * `start_time` - Start timestamp
    /// * `end_time` - End timestamp
    /// * `limit` - Maximum results (max 100)
    ///
    /// # Returns
    /// Vector of payments created within the time range
    pub fn get_payments_by_time_range(
        env: Env,
        start_time: u64,
        end_time: u64,
        limit: u64,
    ) -> Vec<Payment> {
        let max_limit = if limit > 100 { 100 } else { limit };
        let mut results = Vec::new(&env);
        let total_count = env.storage().instance().get(&PAYMENT_COUNT).unwrap_or(0u64);

        for payment_id in 1..=total_count {
            if results.len() >= max_limit as u32 {
                break;
            }

            let key = common::StorageKey::Payment(payment_id);
            if let Some(payment) = env.storage().persistent().get::<_, Payment>(&key) {
                if payment.created_at >= start_time && payment.created_at <= end_time {
                    results.push_back(payment);
                }
            }
        }

        results
    }
}
