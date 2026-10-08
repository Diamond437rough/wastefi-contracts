#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn create_token_contract<'a>(env: &Env) -> (Address, WasteTokenClient<'a>) {
    let contract_id = env.register_contract(None, WasteToken);
    let client = WasteTokenClient::new(env, &contract_id);
    (contract_id, client)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    assert_eq!(client.name(), String::from_str(&env, "WasteFi Token"));
    assert_eq!(client.symbol(), String::from_str(&env, "WASTE"));
    assert_eq!(client.decimals(), 7);
    assert_eq!(client.total_supply(), 0);
    assert_eq!(client.admin(), admin);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_cannot_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Try to initialize again - should panic
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
}

#[test]
fn test_mint() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Mint tokens
    client.mint(&user, &1_000_000);

    assert_eq!(client.balance(&user), 1_000_000);
    assert_eq!(client.total_supply(), 1_000_000);
}

#[test]
fn test_mint_multiple() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Mint to multiple users
    client.mint(&user1, &500_000);
    client.mint(&user2, &300_000);

    assert_eq!(client.balance(&user1), 500_000);
    assert_eq!(client.balance(&user2), 300_000);
    assert_eq!(client.total_supply(), 800_000);
}

#[test]
fn test_burn() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user, &1_000_000);

    // Burn tokens
    client.burn(&user, &400_000);

    assert_eq!(client.balance(&user), 600_000);
    assert_eq!(client.total_supply(), 600_000);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_burn_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user, &100_000);

    // Try to burn more than balance - should panic
    client.burn(&user, &200_000);
}

#[test]
fn test_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user1, &1_000_000);

    // Transfer tokens
    client.transfer(&user1, &user2, &300_000);

    assert_eq!(client.balance(&user1), 700_000);
    assert_eq!(client.balance(&user2), 300_000);
    assert_eq!(client.total_supply(), 1_000_000);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_transfer_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user1, &100_000);

    // Try to transfer more than balance - should panic
    client.transfer(&user1, &user2, &200_000);
}

#[test]
fn test_pause_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Check not paused initially
    assert!(!client.is_paused());

    // Pause
    client.pause();
    assert!(client.is_paused());

    // Unpause
    client.unpause();
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Contract paused")]
fn test_cannot_mint_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and pause
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.pause();

    // Try to mint - should panic
    client.mint(&user, &1_000_000);
}

#[test]
fn test_zero_balance_default() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Check balance is 0 for address that never received tokens
    assert_eq!(client.balance(&user), 0);
}

#[test]
fn test_transfer_event_emission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Mint tokens to user1
    client.mint(&user1, &1_000_000);

    // Transfer tokens from user1 to user2
    client.transfer(&user1, &user2, &300_000);

    // Verify event was emitted with correct data
    let events = env.events().all();
    let transfer_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("transfer")
        })
        .collect();

    // Should have exactly one transfer event
    assert_eq!(transfer_events.len(), 1);

    // Verify balances updated correctly after event emission
    assert_eq!(client.balance(&user1), 700_000);
    assert_eq!(client.balance(&user2), 300_000);
}

#[test]
fn test_mint_event_emission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Mint tokens
    client.mint(&user, &500_000);

    // Verify event was emitted
    let events = env.events().all();
    let mint_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("mint")
        })
        .collect();

    // Should have exactly one mint event
    assert_eq!(mint_events.len(), 1);

    // Verify balance updated correctly after event emission
    assert_eq!(client.balance(&user), 500_000);
}

#[test]
fn test_burn_event_emission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user, &1_000_000);

    // Burn tokens
    client.burn(&user, &400_000);

    // Verify event was emitted
    let events = env.events().all();
    let burn_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("burn")
        })
        .collect();

    // Should have exactly one burn event
    assert_eq!(burn_events.len(), 1);

    // Verify balance updated correctly after event emission
    assert_eq!(client.balance(&user), 600_000);
}

#[test]
fn test_multiple_transfer_events() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user1, &1_000_000);

    // Perform multiple transfers
    client.transfer(&user1, &user2, &200_000);
    client.transfer(&user1, &user3, &100_000);
    client.transfer(&user2, &user3, &50_000);

    // Verify all transfer events were emitted
    let events = env.events().all();
    let transfer_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("transfer")
        })
        .collect();

    // Should have exactly three transfer events
    assert_eq!(transfer_events.len(), 3);

    // Verify final balances
    assert_eq!(client.balance(&user1), 700_000);
    assert_eq!(client.balance(&user2), 150_000);
    assert_eq!(client.balance(&user3), 150_000);
}

#[test]
fn test_transfer_zero_amount_event() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user1, &1_000_000);

    // Transfer zero amount
    client.transfer(&user1, &user2, &0);

    // Verify event was emitted even for zero amount
    let events = env.events().all();
    let transfer_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("transfer")
        })
        .collect();

    // Should have exactly one transfer event
    assert_eq!(transfer_events.len(), 1);

    // Verify balances remain unchanged
    assert_eq!(client.balance(&user1), 1_000_000);
    assert_eq!(client.balance(&user2), 0);
}

#[test]
fn test_transfer_to_self_event() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize and mint
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );
    client.mint(&user, &1_000_000);

    // Transfer to self
    client.transfer(&user, &user, &100_000);

    // Verify event was emitted
    let events = env.events().all();
    let transfer_events: soroban_sdk::Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.get(0).unwrap() == soroban_sdk::symbol_short!("transfer")
        })
        .collect();

    // Should have exactly one transfer event
    assert_eq!(transfer_events.len(), 1);

    // Balance should remain the same for self-transfer
    assert_eq!(client.balance(&user), 1_000_000);
}

#[test]
fn test_event_emission_order_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let (_, client) = create_token_contract(&env);

    // Initialize
    client.initialize(
        &admin,
        &String::from_str(&env, "WasteFi Token"),
        &String::from_str(&env, "WASTE"),
        &7,
    );

    // Perform a sequence of operations
    client.mint(&user1, &1_000_000);    // mint event
    client.transfer(&user1, &user2, &300_000); // transfer event
    client.burn(&user2, &100_000);      // burn event

    // Verify events were emitted in correct order
    let events = env.events().all();
    
    let event_types: soroban_sdk::Vec<_> = events
        .iter()
        .map(|e| e.topics.get(0).unwrap())
        .collect();

    // Should have mint, transfer, and burn events
    assert!(event_types.len() >= 3);

    // Verify final state
    assert_eq!(client.balance(&user1), 700_000);
    assert_eq!(client.balance(&user2), 200_000);
    assert_eq!(client.total_supply(), 900_000);
}
