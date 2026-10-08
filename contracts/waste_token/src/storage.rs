use soroban_sdk::{Address, Env, String};

/// Storage keys
const TOKEN_NAME: &str = "TokenName";
const TOKEN_SYMBOL: &str = "TokenSymbol";
const TOKEN_DECIMALS: &str = "TokenDecimals";
const TOTAL_SUPPLY: &str = "TotalSupply";

/// Write token metadata
pub fn write_metadata(env: &Env, name: String, symbol: String, decimals: u32) {
    env.storage().instance().set(&TOKEN_NAME, &name);
    env.storage().instance().set(&TOKEN_SYMBOL, &symbol);
    env.storage().instance().set(&TOKEN_DECIMALS, &decimals);
}

/// Read token name
pub fn read_name(env: &Env) -> String {
    env.storage()
        .instance()
        .get(&TOKEN_NAME)
        .unwrap_or(String::from_str(env, ""))
}

/// Read token symbol
pub fn read_symbol(env: &Env) -> String {
    env.storage()
        .instance()
        .get(&TOKEN_SYMBOL)
        .unwrap_or(String::from_str(env, ""))
}

/// Read token decimals
pub fn read_decimals(env: &Env) -> u32 {
    env.storage().instance().get(&TOKEN_DECIMALS).unwrap_or(7)
}

/// Write total supply
pub fn write_total_supply(env: &Env, supply: i128) {
    env.storage().instance().set(&TOTAL_SUPPLY, &supply);
}

/// Read total supply
pub fn read_total_supply(env: &Env) -> i128 {
    env.storage().instance().get(&TOTAL_SUPPLY).unwrap_or(0)
}

/// Write balance for an address
pub fn write_balance(env: &Env, address: &Address, balance: i128) {
    let key = ("Balance", address);
    env.storage().persistent().set(&key, &balance);

    // Bump storage TTL
    let storage_key = common::StorageKey::Balance(address.clone());
    common::bump_persistent(env, &storage_key);
}

/// Read balance for an address
pub fn read_balance(env: &Env, address: &Address) -> i128 {
    let key = ("Balance", address);
    env.storage().persistent().get(&key).unwrap_or(0)
}

/// Write allowance for a spender
pub fn write_allowance(env: &Env, owner: &Address, spender: &Address, amount: i128) {
    let key = ("Allowance", owner, spender);
    env.storage().persistent().set(&key, &amount);

    // Bump storage TTL
    let storage_key = common::StorageKey::Allowance(owner.clone(), spender.clone());
    common::bump_persistent(env, &storage_key);
}

/// Read allowance for a spender
pub fn read_allowance(env: &Env, owner: &Address, spender: &Address) -> i128 {
    let key = ("Allowance", owner, spender);
    env.storage().persistent().get(&key).unwrap_or(0)
}
