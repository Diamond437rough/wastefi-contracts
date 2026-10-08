#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String};

mod storage;
use storage::*;

#[cfg(test)]
mod test;

#[contract]
pub struct WasteToken;

#[contractimpl]
impl WasteToken {
    /// Initialize the token contract
    ///
    /// # Arguments
    /// * `admin` - Contract administrator address
    /// * `name` - Token name (e.g., "WasteFi Token")
    /// * `symbol` - Token symbol (e.g., "WASTE")
    /// * `decimals` - Number of decimal places (typically 7 for Stellar)
    pub fn initialize(env: Env, admin: Address, name: String, symbol: String, decimals: u32) {
        common::Initializable::require_not_initialized(&env).expect("Already initialized");

        // Validate inputs
        common::validation::validate_non_empty_string(&name).expect("Invalid name");
        common::validation::validate_non_empty_string(&symbol).expect("Invalid symbol");

        // Set admin
        common::AccessControl::set_admin(&env, admin.clone());

        // Store token metadata
        write_metadata(&env, name, symbol, decimals);

        // Initialize total supply to 0
        write_total_supply(&env, 0);

        // Mark as initialized
        common::Initializable::mark_initialized(&env);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Mint new tokens (admin only)
    ///
    /// # Arguments
    /// * `to` - Recipient address
    /// * `amount` - Amount to mint
    pub fn mint(env: Env, to: Address, amount: i128) {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Get admin and verify
        let admin = common::AccessControl::get_admin(&env).expect("Admin not found");
        common::AccessControl::require_admin(&env, &admin).expect("Not admin");

        // Validate amount
        common::validate_amount(amount).expect("Invalid amount");

        // Emit event before storage updates for consistency
        common::TokenEvents::mint(&env, to.clone(), amount);

        // Get current balance
        let current_balance = read_balance(&env, &to);
        let new_balance = current_balance.saturating_add(amount);

        // Update balance
        write_balance(&env, &to, new_balance);

        // Update total supply
        let total_supply = read_total_supply(&env);
        let new_total_supply = total_supply.saturating_add(amount);
        write_total_supply(&env, new_total_supply);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Burn tokens
    ///
    /// # Arguments
    /// * `from` - Address to burn from
    /// * `amount` - Amount to burn
    pub fn burn(env: Env, from: Address, amount: i128) {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Require authorization from the account burning
        from.require_auth();

        // Validate amount
        common::validate_amount(amount).expect("Invalid amount");

        // Get current balance
        let current_balance = read_balance(&env, &from);

        // Check sufficient balance
        if current_balance < amount {
            panic!("Insufficient balance");
        }

        let new_balance = current_balance - amount;

        // Emit event before storage updates for consistency
        common::TokenEvents::burn(&env, from.clone(), amount);

        // Update balance
        write_balance(&env, &from, new_balance);

        // Update total supply
        let total_supply = read_total_supply(&env);
        let new_total_supply = total_supply - amount;
        write_total_supply(&env, new_total_supply);

        // Bump storage
        common::bump_instance(&env);
    }

    /// Transfer tokens
    ///
    /// # Arguments
    /// * `from` - Sender address
    /// * `to` - Recipient address
    /// * `amount` - Amount to transfer
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        common::Initializable::require_initialized(&env).expect("Not initialized");
        common::Pausable::require_not_paused(&env).expect("Contract paused");

        // Require authorization from sender
        from.require_auth();

        // Validate amount
        common::validate_amount(amount).expect("Invalid amount");

        // Get balances
        let from_balance = read_balance(&env, &from);
        let to_balance = read_balance(&env, &to);

        // Check sufficient balance
        if from_balance < amount {
            panic!("Insufficient balance");
        }

        // Emit event before storage updates for consistency
        common::TokenEvents::transfer(&env, from.clone(), to.clone(), amount);

        // Update balances
        write_balance(&env, &from, from_balance - amount);
        write_balance(&env, &to, to_balance.saturating_add(amount));

        // Bump storage
        common::bump_instance(&env);
    }

    /// Get token balance for an address
    ///
    /// # Arguments
    /// * `account` - Address to check balance
    ///
    /// # Returns
    /// Token balance
    pub fn balance(env: Env, account: Address) -> i128 {
        read_balance(&env, &account)
    }

    /// Get total token supply
    ///
    /// # Returns
    /// Total supply
    pub fn total_supply(env: Env) -> i128 {
        read_total_supply(&env)
    }

    /// Get token name
    ///
    /// # Returns
    /// Token name
    pub fn name(env: Env) -> String {
        read_name(&env)
    }

    /// Get token symbol
    ///
    /// # Returns
    /// Token symbol
    pub fn symbol(env: Env) -> String {
        read_symbol(&env)
    }

    /// Get token decimals
    ///
    /// # Returns
    /// Number of decimals
    pub fn decimals(env: Env) -> u32 {
        read_decimals(&env)
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
}
