# WasteFi Testing Guide for Contributors

Welcome to the WasteFi testing guide! This document will help you write effective tests for smart contracts in the WasteFi ecosystem. Whether you're new to Soroban or an experienced developer, this guide covers everything you need to know about testing in this project.

**Version**: 1.0.0  
**Last Updated**: October 2026  
**Target Audience**: Contributors, developers, and testers

---

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Test Structure Overview](#2-test-structure-overview)
3. [Writing Unit Tests](#3-writing-unit-tests)
4. [Writing Integration Tests](#4-writing-integration-tests)
5. [Property-Based Testing](#5-property-based-testing)
6. [Running Tests](#6-running-tests)
7. [Test Coverage](#7-test-coverage)
8. [Best Practices](#8-best-practices)
9. [Common Patterns](#9-common-patterns)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Getting Started

### 1.1 Prerequisites

Before writing tests, ensure you have the following installed:

```bash
# Rust (1.79.0 or later)
rustup --version

# Soroban SDK
cargo --version

# wasm32 target for contract compilation
rustup target add wasm32-unknown-unknown
```

### 1.2 Project Structure

Tests in WasteFi are organized into three main categories:

```
wastefi-contracts/
├── contracts/
│   ├── waste_token/
│   │   └── src/
│   │       ├── lib.rs          # Contract implementation
│   │       └── test.rs         # Unit tests
│   ├── collector_registry/
│   │   └── src/
│   │       └── test.rs
│   └── ...
└── tests/
    ├── integration_e2e.rs      # Integration tests
    ├── stress_tests.rs         # Stress/performance tests
    └── ...
```

### 1.3 Test Philosophy

The WasteFi project follows these testing principles:

1. **Test Early, Test Often**: Write tests alongside your code
2. **Comprehensive Coverage**: Aim for >85% code coverage
3. **Test Pyramid**: Many unit tests, some integration tests, few E2E tests
4. **Security First**: Test all security controls and edge cases
5. **Clear and Maintainable**: Write tests that are easy to understand and maintain

---

## 2. Test Structure Overview

### 2.1 Test Types

#### Unit Tests
- **Purpose**: Test individual functions in isolation
- **Location**: `src/test.rs` or inline in `src/lib.rs`
- **Scope**: Single contract, single function
- **Speed**: Fast (~milliseconds per test)

#### Integration Tests
- **Purpose**: Test interactions between multiple contracts
- **Location**: `tests/` directory
- **Scope**: Multiple contracts working together
- **Speed**: Moderate (~seconds per test)

#### Property-Based Tests
- **Purpose**: Test properties that should always hold true
- **Location**: Can be in unit or integration tests
- **Scope**: Test many inputs to verify invariants
- **Speed**: Slower (many test cases per property)

### 2.2 Test Anatomy

A typical test follows the **Arrange-Act-Assert** pattern:

```rust
#[test]
fn test_example() {
    // ARRANGE: Set up test environment and data
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    
    // ACT: Execute the function being tested
    let result = contract.some_function(&admin);
    
    // ASSERT: Verify the result
    assert_eq!(result, expected_value);
}
```

---

## 3. Writing Unit Tests

Unit tests focus on testing individual contract functions in isolation.

### 3.1 Basic Unit Test Setup

Every unit test starts with the same basic setup:

```rust
#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// Helper function to create contract instance
fn create_token_contract<'a>(env: &Env) -> (Address, WasteTokenClient<'a>) {
    let contract_id = env.register_contract(None, WasteToken);
    let client = WasteTokenClient::new(env, &contract_id);
    (contract_id, client)
}
```

**Key Components**:
- `#![cfg(test)]`: Only compiles test code in test mode
- `use super::*`: Imports contract code
- `Env::default()`: Creates test environment
- `env.mock_all_auths()`: Bypasses authorization checks for testing

### 3.2 Testing Happy Paths

Start with testing the expected, successful behavior:

```rust
#[test]
fn test_initialize() {
    // Setup
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    // Execute
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    
    // Verify all expected state
    assert_eq!(client.name(), String::from_str(&env, "WasteFi Token"));
    assert_eq!(client.symbol(), String::from_str(&env, "WASTE"));
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.total_supply(), 0);
    assert_eq!(client.admin(), admin);
}
```

### 3.3 Testing Error Conditions

Always test that your contract properly handles and rejects invalid inputs:

```rust
#[test]
#[should_panic(expected = "Already initialized")]
fn test_cannot_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    // First initialization succeeds
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    
    // Second initialization should panic
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
}

#[test]
#[should_panic(expected = "Invalid amount")]
fn test_transfer_negative_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    
    // Attempting to transfer negative amount should fail
    client.transfer(&user1, &user2, &-100);
}
```

### 3.4 Testing Authorization

Test that functions properly enforce authorization requirements:

```rust
#[test]
fn test_mint_requires_admin() {
    let env = Env::default();
    
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    // Mock only admin authorization
    env.mock_all_auths();
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    
    // Clear mock auths and set specific authorization
    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "mint",
                args: (user.clone(), 1000i128).into_val(&env),
                sub_invokes: &[],
            },
        },
    ]);
    
    // Admin can mint (should succeed)
    client.mint(&user, &1000);
    assert_eq!(client.balance(&user), 1000);
}
```

### 3.5 Testing State Changes

Verify that functions correctly modify contract state:

```rust
#[test]
fn test_transfer_updates_balances() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    // Setup
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client.mint(&user1, &1000);
    
    // Record initial state
    let initial_user1_balance = client.balance(&user1);
    let initial_user2_balance = client.balance(&user2);
    
    // Execute transfer
    client.transfer(&user1, &user2, &300);
    
    // Verify state changes
    assert_eq!(client.balance(&user1), initial_user1_balance - 300);
    assert_eq!(client.balance(&user2), initial_user2_balance + 300);
    assert_eq!(client.total_supply(), 1000); // Total supply unchanged
}
```

### 3.6 Testing Edge Cases

Test boundary values and edge conditions:

```rust
#[test]
fn test_transfer_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client.mint(&user1, &1000);
    
    // Zero transfer should succeed without error
    client.transfer(&user1, &user2, &0);
    
    assert_eq!(client.balance(&user1), 1000);
    assert_eq!(client.balance(&user2), 0);
}

#[test]
fn test_transfer_exact_balance() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client.mint(&user1, &1000);
    
    // Transfer entire balance should succeed
    client.transfer(&user1, &user2, &1000);
    
    assert_eq!(client.balance(&user1), 0);
    assert_eq!(client.balance(&user2), 1000);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_transfer_exceeds_balance() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client.mint(&user1, &1000);
    
    // Transfer more than balance should fail
    client.transfer(&user1, &user2, &1001);
}
```

---

## 4. Writing Integration Tests

Integration tests verify that multiple contracts work together correctly.

### 4.1 Integration Test Setup

Integration tests are located in the `tests/` directory and have a different structure:

```rust
// tests/integration_test.rs
#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use waste_token::{WasteToken, WasteTokenClient};
use collector_registry::{CollectorRegistry, CollectorRegistryClient};
use waste_transaction::{WasteTransaction, WasteTransactionClient};

#[test]
fn test_complete_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    
    // Register all contracts
    let token_id = env.register_contract(None, WasteToken);
    let registry_id = env.register_contract(None, CollectorRegistry);
    let transaction_id = env.register_contract(None, WasteTransaction);
    
    // Create clients
    let token = WasteTokenClient::new(&env, &token_id);
    let registry = CollectorRegistryClient::new(&env, &registry_id);
    let transaction = WasteTransactionClient::new(&env, &transaction_id);
    
    // Test workflow across contracts
    // ... test implementation
}
```

### 4.2 Testing Cross-Contract Interactions

```rust
#[test]
fn test_transaction_triggers_payment() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let collector = Address::generate(&env);
    
    // Setup contracts
    let token = setup_token(&env, &admin);
    let registry = setup_registry(&env, &admin);
    let transaction_contract = setup_transaction(&env, &admin, &token_id, &registry_id);
    
    // Register collector
    registry.register(
        &collector,
        &String::from_str(&env, "Test Collector"),
        &String::from_str(&env, "test@example.com"),
    );
    
    // Record transaction
    let tx_id = transaction_contract.record(
        &collector,
        &1u32, // MaterialType::Plastic
        &5000u64, // weight in grams
        &String::from_str(&env, "LOC123"),
    );
    
    // Verify transaction was recorded
    let tx_info = transaction_contract.get_transaction(&tx_id);
    assert_eq!(tx_info.collector, collector);
    assert_eq!(tx_info.weight, 5000);
    
    // Verify collector received tokens
    let balance = token.balance(&collector);
    assert!(balance > 0, "Collector should receive tokens");
}
```

### 4.3 Testing Error Propagation

Test that errors in one contract are properly handled by calling contracts:

```rust
#[test]
fn test_transaction_fails_with_unregistered_collector() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let unregistered_collector = Address::generate(&env);
    
    // Setup contracts
    let token = setup_token(&env, &admin);
    let registry = setup_registry(&env, &admin);
    let transaction_contract = setup_transaction(&env, &admin, &token_id, &registry_id);
    
    // Try to record transaction with unregistered collector
    let result = transaction_contract.try_record(
        &unregistered_collector,
        &1u32,
        &5000u64,
        &String::from_str(&env, "LOC123"),
    );
    
    // Should fail because collector is not registered
    assert!(result.is_err());
}
```

### 4.4 Testing State Consistency Across Contracts

```rust
#[test]
fn test_state_consistency_after_multiple_operations() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let collector1 = Address::generate(&env);
    let collector2 = Address::generate(&env);
    
    // Setup all contracts
    let token = setup_token(&env, &admin);
    let registry = setup_registry(&env, &admin);
    let transaction_contract = setup_transaction(&env, &admin, &token_id, &registry_id);
    let reputation = setup_reputation(&env, &admin);
    
    // Register collectors
    registry.register(&collector1, &String::from_str(&env, "Collector 1"), &String::from_str(&env, "c1@test.com"));
    registry.register(&collector2, &String::from_str(&env, "Collector 2"), &String::from_str(&env, "c2@test.com"));
    
    // Record transactions
    transaction_contract.record(&collector1, &1u32, &5000u64, &String::from_str(&env, "LOC1"));
    transaction_contract.record(&collector2, &1u32, &3000u64, &String::from_str(&env, "LOC2"));
    transaction_contract.record(&collector1, &2u32, &2000u64, &String::from_str(&env, "LOC3"));
    
    // Verify consistent state across all contracts
    let total_supply = token.total_supply();
    let collector1_balance = token.balance(&collector1);
    let collector2_balance = token.balance(&collector2);
    
    assert_eq!(total_supply, collector1_balance + collector2_balance, 
               "Total supply should equal sum of balances");
    
    // Verify transaction count
    let tx_count = transaction_contract.get_transaction_count();
    assert_eq!(tx_count, 3, "Should have 3 transactions recorded");
}
```

---

## 5. Property-Based Testing

Property-based testing verifies that certain properties always hold true, regardless of input.

### 5.1 What is Property-Based Testing?

Instead of testing specific examples, property-based tests verify invariants:

```rust
// Example: The total supply should always equal the sum of all balances
#[test]
fn property_total_supply_equals_balance_sum() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    
    // Generate random addresses and amounts
    let mut total_minted = 0i128;
    for _ in 0..10 {
        let user = Address::generate(&env);
        let amount = (env.ledger().sequence() as i128) * 100; // Pseudo-random
        client.mint(&user, &amount);
        total_minted += amount;
    }
    
    // Property: total supply should equal total minted
    assert_eq!(client.total_supply(), total_minted);
}
```

### 5.2 Common Properties to Test

#### Conservation Properties
```rust
#[test]
fn property_transfer_preserves_total_supply() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    client.mint(&user1, &1000);
    let initial_supply = client.total_supply();
    
    // Transfer any amount
    client.transfer(&user1, &user2, &300);
    
    // Property: total supply unchanged after transfer
    assert_eq!(client.total_supply(), initial_supply);
}
```

#### Idempotency Properties
```rust
#[test]
fn property_reading_balance_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);
    
    client.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client.mint(&user, &1000);
    
    // Property: reading balance multiple times returns same value
    let balance1 = client.balance(&user);
    let balance2 = client.balance(&user);
    let balance3 = client.balance(&user);
    
    assert_eq!(balance1, balance2);
    assert_eq!(balance2, balance3);
}
```

#### Ordering Properties
```rust
#[test]
fn property_operations_order_matters() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    // Test case 1: mint then transfer
    let (_, client1) = create_token_contract(&env);
    client1.initialize(&admin, &String::from_str(&env, "Test"), &String::from_str(&env, "TST"), &7);
    client1.mint(&user, &1000);
    client1.transfer(&user, &admin, &500);
    let balance1 = client1.balance(&user);
    
    // Test case 2: different order should give different result
    // This tests that operations have proper ordering requirements
    assert_eq!(balance1, 500);
}
```

---

## 6. Running Tests

### 6.1 Running All Tests

```bash
# Run all tests in the workspace
cargo test --workspace

# Run with output visible
cargo test --workspace -- --nocapture

# Run with specific number of threads
cargo test --workspace -- --test-threads=4
```

### 6.2 Running Specific Tests

```bash
# Run tests for a specific contract
cargo test -p waste_token

# Run a specific test by name
cargo test test_initialize --workspace

# Run tests matching a pattern
cargo test transfer --workspace
```

### 6.3 Running Integration Tests

```bash
# Run only integration tests
cargo test --test integration_e2e

# Run all integration tests
cargo test --tests
```

### 6.4 Running Tests in Release Mode

```bash
# Faster execution for large test suites
cargo test --workspace --release
```

### 6.5 Filtering Test Output

```bash
# Show only failed tests
cargo test --workspace --quiet

# Show test names as they run
cargo test --workspace -- --show-output

# Limit test execution time
cargo test --workspace -- --test-timeout 30
```

---

## 7. Test Coverage

### 7.1 Generating Coverage Reports

Install `cargo-tarpaulin`:
```bash
cargo install cargo-tarpaulin
```

Generate coverage:
```bash
# HTML report
cargo tarpaulin --workspace --out Html --output-dir coverage

# Open report (Windows)
start coverage\index.html

# Open report (Linux/Mac)
open coverage/index.html
```

### 7.2 Coverage Requirements

WasteFi targets **>85% code coverage**:

```bash
# Check if coverage meets threshold
cargo tarpaulin --workspace --fail-under 85
```

### 7.3 Interpreting Coverage

Coverage reports show:
- **Green lines**: Executed by tests
- **Red lines**: Not executed by tests
- **Yellow lines**: Partially executed (e.g., some branches)

Focus on covering:
1. All public functions
2. Error handling paths
3. Edge cases and boundaries
4. Security-critical code

### 7.4 Coverage Best Practices

**DO**:
- ✅ Aim for high coverage on critical paths
- ✅ Test both success and failure cases
- ✅ Cover edge cases and boundaries
- ✅ Track coverage trends over time

**DON'T**:
- ❌ Focus solely on achieving 100% coverage
- ❌ Ignore testing quality for coverage numbers
- ❌ Skip testing because coverage is already high
- ❌ Test private implementation details just for coverage

---

## 8. Best Practices

### 8.1 Test Naming

Use descriptive names that explain what is being tested:

```rust
// ❌ Bad: Unclear what is being tested
#[test]
fn test1() { }

// ✅ Good: Clear and descriptive
#[test]
fn test_transfer_reduces_sender_balance() { }

// ✅ Good: Describes failure case
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_transfer_fails_when_balance_insufficient() { }
```

### 8.2 Test Organization

Group related tests using modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod initialization {
        use super::*;
        
        #[test]
        fn test_initialize_sets_metadata() { }
        
        #[test]
        fn test_cannot_initialize_twice() { }
    }
    
    mod transfers {
        use super::*;
        
        #[test]
        fn test_transfer_happy_path() { }
        
        #[test]
        fn test_transfer_insufficient_balance() { }
    }
    
    mod authorization {
        use super::*;
        
        #[test]
        fn test_admin_only_functions() { }
    }
}
```

### 8.3 Test Independence

Each test should be independent and not rely on other tests:

```rust
// ❌ Bad: Tests depend on each other
static mut SHARED_STATE: i32 = 0;

#[test]
fn test_a() {
    unsafe { SHARED_STATE = 5; }
}

#[test]
fn test_b() {
    // This test depends on test_a running first
    unsafe { assert_eq!(SHARED_STATE, 5); }
}

// ✅ Good: Each test sets up its own state
#[test]
fn test_a() {
    let state = 5;
    assert_eq!(state, 5);
}

#[test]
fn test_b() {
    let state = 5;
    assert_eq!(state, 5);
}
```

### 8.4 Use Helper Functions

Reduce code duplication with helper functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function for common setup
    fn setup_initialized_token(env: &Env) -> (Address, WasteTokenClient) {
        let admin = Address::generate(env);
        let (_, client) = create_token_contract(env);
        
        client.initialize(
            &admin,
            &String::from_str(env, "WasteFi Token"),
            &String::from_str(env, "WASTE"),
            &7,
        );
        
        (admin, client)
    }
    
    // Helper function to mint tokens to a user
    fn mint_to_user(client: &WasteTokenClient, user: &Address, amount: i128) {
        client.mint(user, &amount);
    }
    
    #[test]
    fn test_with_helpers() {
        let env = Env::default();
        env.mock_all_auths();
        
        let (admin, client) = setup_initialized_token(&env);
        let user = Address::generate(&env);
        
        mint_to_user(&client, &user, 1000);
        
        assert_eq!(client.balance(&user), 1000);
    }
}
```

### 8.5 Testing Error Messages

Verify specific error messages when testing failures:

```rust
#[test]
#[should_panic(expected = "Invalid amount")]
fn test_negative_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (admin, client) = setup_initialized_token(&env);
    let user = Address::generate(&env);
    
    // Should panic with specific error message
    client.mint(&user, &-100);
}
```

### 8.6 Documentation

Document complex test scenarios:

```rust
/// Tests that the allowance system correctly handles the approve-transfer_from workflow
/// 
/// Scenario:
/// 1. User A approves User B to spend 500 tokens
/// 2. User B transfers 300 tokens from User A to User C
/// 3. User B's remaining allowance should be 200
/// 4. User B attempts to transfer 250 more tokens and should fail
#[test]
fn test_allowance_workflow() {
    // Test implementation
}
```

---

## 9. Common Patterns

### 9.1 Mocking Authorization

```rust
// Mock all authorizations (simplest, for most tests)
env.mock_all_auths();

// Mock specific authorization
env.mock_auths(&[
    MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "transfer",
            args: (recipient.clone(), 100i128).into_val(&env),
            sub_invokes: &[],
        },
    },
]);
```

### 9.2 Generating Test Addresses

```rust
use soroban_sdk::testutils::Address as _;

let address1 = Address::generate(&env);
let address2 = Address::generate(&env);
```

### 9.3 Creating Test Strings

```rust
use soroban_sdk::String;

let name = String::from_str(&env, "Test Name");
let symbol = String::from_str(&env, "TST");
```

### 9.4 Testing Events

```rust
#[test]
fn test_transfer_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (admin, client) = setup_initialized_token(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    client.mint(&user1, &1000);
    
    // Transfer will emit an event
    client.transfer(&user1, &user2, &500);
    
    // Events are automatically published to env.events()
    // In integration tests, you can verify event emission
}
```

### 9.5 Testing with Time

```rust
#[test]
fn test_time_dependent_function() {
    let env = Env::default();
    env.mock_all_auths();
    
    // Set ledger sequence (block height)
    env.ledger().set_sequence_number(100);
    
    // Set ledger timestamp
    env.ledger().set_timestamp(1234567890);
    
    // Run time-dependent test
    // ...
}
```

---

## 10. Troubleshooting

### 10.1 Common Test Errors

#### "NotAuthorized" Error
```
Error: Status(ContractError(2)) // NotAuthorized
```

**Solution**: Add `env.mock_all_auths()` before function calls:
```rust
let env = Env::default();
env.mock_all_auths(); // Add this line
```

#### "Already initialized" Error
```
Error: Already initialized
```

**Solution**: Each test should create a new contract instance:
```rust
#[test]
fn test_something() {
    let env = Env::default();
    let (_, client) = create_token_contract(&env); // New instance per test
    // ...
}
```

#### Test Panics Without Message
```
thread 'test_name' panicked at ...
```

**Solution**: Run test with `--nocapture` to see full output:
```bash
cargo test test_name -- --nocapture
```

#### Compilation Errors with wasm32 Target
```
error: could not compile ...
```

**Solution**: Ensure wasm32 target is installed:
```bash
rustup target add wasm32-unknown-unknown
```

### 10.2 Debugging Tests

#### Print Debug Information
```rust
#[test]
fn test_with_debugging() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (admin, client) = setup_initialized_token(&env);
    
    // Print values during test
    println!("Admin address: {:?}", admin);
    println!("Total supply: {}", client.total_supply());
    
    // Test continues...
}
```

Run with:
```bash
cargo test test_with_debugging -- --nocapture
```

#### Use assert! with Messages
```rust
let balance = client.balance(&user);
assert!(balance > 0, "Balance should be positive, got: {}", balance);
```

### 10.3 Performance Issues

If tests are slow:

```bash
# Run in release mode
cargo test --release

# Reduce parallelism if tests conflict
cargo test -- --test-threads=1

# Run specific test suites only
cargo test -p waste_token
```

### 10.4 Getting Help

When stuck:
1. Check the [Soroban documentation](https://soroban.stellar.org/)
2. Review existing tests in the codebase
3. Look at `tests/integration_e2e.rs` for examples
4. Check the main `TESTING.md` document
5. Open an issue on GitHub with test output

---

## Summary

### Quick Reference

**Writing a Unit Test**:
```rust
#[test]
fn test_function_name() {
    let env = Env::default();
    env.mock_all_auths();
    
    // Setup
    let (admin, client) = setup_contract(&env);
    
    // Execute
    let result = client.some_function(...);
    
    // Verify
    assert_eq!(result, expected_value);
}
```

**Running Tests**:
```bash
cargo test --workspace              # All tests
cargo test -p waste_token          # Specific contract
cargo test test_name               # Specific test
```

**Coverage**:
```bash
cargo tarpaulin --workspace --out Html --output-dir coverage
```

### Before Submitting

Checklist for contributors:
- [ ] All tests pass locally
- [ ] New tests added for new functionality
- [ ] Edge cases and error conditions tested
- [ ] Test names are descriptive
- [ ] Coverage maintained or improved (>85%)
- [ ] Tests are independent and don't rely on order
- [ ] No test warnings or ignored tests

---

## Additional Resources

- **Soroban Documentation**: https://soroban.stellar.org/
- **Rust Testing Guide**: https://doc.rust-lang.org/book/ch11-00-testing.html
- **Project TESTING.md**: Full testing infrastructure documentation
- **CONTRIBUTING.md**: General contribution guidelines

---

**Happy Testing! 🚀**

For questions or feedback, open an issue or contact the WasteFi team.

---

**Document Version**: 1.0.0  
**Last Updated**: October 2026  
**Maintained by**: WasteFi Development Team
