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
fn test_get_max_supply() {
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

    // Check max supply is 1 billion with 7 decimals
    assert_eq!(client.get_max_supply(), 10_000_000_000_000_000i128);
}

#[test]
fn test_get_remaining_supply() {
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

    // Initially all supply is remaining
    assert_eq!(client.get_remaining_supply(), 10_000_000_000_000_000i128);

    // Mint some tokens
    client.mint(&user, &1_000_000_000_000_000i128);

    // Check remaining supply decreased
    assert_eq!(client.get_remaining_supply(), 9_000_000_000_000_000i128);
}

#[test]
#[should_panic(expected = "Minting would exceed maximum supply cap")]
fn test_cannot_exceed_max_supply() {
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

    // Try to mint more than max supply - should panic
    client.mint(&user, &10_000_000_000_000_001i128);
}

#[test]
#[should_panic(expected = "Minting would exceed maximum supply cap")]
fn test_cannot_exceed_max_supply_cumulative() {
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

    // Mint up to max supply
    client.mint(&user, &10_000_000_000_000_000i128);

    // Try to mint even 1 more token - should panic
    client.mint(&user, &1);
}

#[test]
fn test_can_mint_exactly_max_supply() {
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

    // Mint exactly max supply - should succeed
    client.mint(&user, &10_000_000_000_000_000i128);

    assert_eq!(client.total_supply(), 10_000_000_000_000_000i128);
    assert_eq!(client.balance(&user), 10_000_000_000_000_000i128);
    assert_eq!(client.get_remaining_supply(), 0);
}

#[test]
fn test_can_mint_after_burn() {
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

    // Mint up to max supply
    client.mint(&user, &10_000_000_000_000_000i128);

    // Burn some tokens
    client.burn(&user, &1_000_000_000_000_000i128);

    // Should be able to mint again after burning
    client.mint(&user, &500_000_000_000_000i128);

    assert_eq!(client.total_supply(), 9_500_000_000_000_000i128);
    assert_eq!(client.get_remaining_supply(), 500_000_000_000_000i128);
}
