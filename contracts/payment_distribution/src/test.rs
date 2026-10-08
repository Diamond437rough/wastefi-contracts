#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn create_contract(env: &Env) -> (Address, PaymentDistributionClient) {
    let contract_id = env.register_contract(None, PaymentDistribution);
    let client = PaymentDistributionClient::new(env, &contract_id);
    (contract_id, client)
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    assert_eq!(client.admin(), admin);
    assert_eq!(client.get_token_contract(), token_contract);
    assert_eq!(client.get_payment_count(), 0);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_cannot_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);
    client.initialize(&admin, &token_contract); // Should panic
}

#[test]
fn test_process_payment() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process payment for transaction
    let payment_id = client.process_payment(&transaction_contract, &1);

    assert_eq!(payment_id, 1);
    assert_eq!(client.get_payment_count(), 1);

    // Check payment details
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.id, 1);
    assert_eq!(payment.transaction_id, 1);
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.created_at, 1000);
}

#[test]
fn test_process_multiple_payments() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process multiple payments
    let payment1 = client.process_payment(&transaction_contract, &1);
    let payment2 = client.process_payment(&transaction_contract, &2);
    let payment3 = client.process_payment(&transaction_contract, &3);

    assert_eq!(payment1, 1);
    assert_eq!(payment2, 2);
    assert_eq!(payment3, 3);
    assert_eq!(client.get_payment_count(), 3);
}

#[test]
fn test_update_payment_status() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Update status to completed
    env.ledger().with_mut(|li| li.timestamp = 2000);
    client.update_payment_status(&payment_id, &PaymentStatus::Completed);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
    assert_eq!(payment.processed_at, 2000);
}

#[test]
fn test_batch_process() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Batch process 3 transactions
    let mut transaction_ids = Vec::new(&env);
    transaction_ids.push_back(1);
    transaction_ids.push_back(2);
    transaction_ids.push_back(3);

    let payment_ids = client.batch_process(&transaction_contract, &transaction_ids);

    assert_eq!(payment_ids.len(), 3);
    assert_eq!(client.get_payment_count(), 3);

    // Verify all payments were created
    for i in 0..payment_ids.len() {
        if let Some(payment_id) = payment_ids.get(i) {
            let payment = client.get_payment(&payment_id);
            assert_eq!(payment.id, payment_id);
        }
    }
}

#[test]
fn test_get_recipient_payments() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process payments (all go to admin as recipient in this simplified version)
    client.process_payment(&transaction_contract, &1);
    client.process_payment(&transaction_contract, &2);
    client.process_payment(&transaction_contract, &3);

    // Get admin's payments
    let payments = client.get_recipient_payments(&admin, &10);
    assert_eq!(payments.len(), 3);
}

#[test]
fn test_limit_recipient_payments() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process 10 payments
    for i in 1..=10 {
        client.process_payment(&transaction_contract, &i);
    }

    // Request only 5
    let payments = client.get_recipient_payments(&admin, &5);
    assert_eq!(payments.len(), 5);

    // Request all (0 means use default limit)
    let all_payments = client.get_recipient_payments(&admin, &0);
    assert_eq!(all_payments.len(), 10);
}

#[test]
fn test_pause_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

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
fn test_cannot_process_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);
    client.pause();

    // Try to process payment while paused
    client.process_payment(&transaction_contract, &1);
}

#[test]
fn test_payment_status_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Check initial status
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Pending);

    // Update to processing
    client.update_payment_status(&payment_id, &PaymentStatus::Processing);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Processing);

    // Update to completed
    client.update_payment_status(&payment_id, &PaymentStatus::Completed);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_payment_status_failed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Mark payment as failed
    client.update_payment_status(&payment_id, &PaymentStatus::Failed);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Failed);
}

#[test]
#[should_panic(expected = "Payment not found")]
fn test_get_nonexistent_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Try to get payment that doesn't exist
    client.get_payment(&999);
}

#[test]
#[should_panic(expected = "Cannot modify completed payment")]
fn test_cannot_modify_completed_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Complete the payment
    client.update_payment_status(&payment_id, &PaymentStatus::Completed);

    // Try to modify completed payment - should panic
    client.update_payment_status(&payment_id, &PaymentStatus::Pending);
}

#[test]
#[should_panic(expected = "Cannot modify failed payment")]
fn test_cannot_modify_failed_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Mark payment as failed
    client.update_payment_status(&payment_id, &PaymentStatus::Failed);

    // Try to modify failed payment - should panic
    client.update_payment_status(&payment_id, &PaymentStatus::Completed);
}

#[test]
#[should_panic(expected = "Cannot modify completed payment")]
fn test_cannot_change_completed_to_processing() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Complete the payment
    client.update_payment_status(&payment_id, &PaymentStatus::Completed);

    // Try to change to processing - should panic
    client.update_payment_status(&payment_id, &PaymentStatus::Processing);
}

#[test]
#[should_panic(expected = "Cannot modify failed payment")]
fn test_cannot_change_failed_to_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    let payment_id = client.process_payment(&transaction_contract, &1);

    // Mark payment as failed
    client.update_payment_status(&payment_id, &PaymentStatus::Failed);

    // Try to change to pending - should panic
    client.update_payment_status(&payment_id, &PaymentStatus::Pending);
}

#[test]
fn test_valid_status_transitions_allowed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let transaction_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Test transition from Pending to Processing (should work)
    let payment_id1 = client.process_payment(&transaction_contract, &1);
    client.update_payment_status(&payment_id1, &PaymentStatus::Processing);
    let payment1 = client.get_payment(&payment_id1);
    assert_eq!(payment1.status, PaymentStatus::Processing);

    // Test transition from Pending to Failed (should work)
    let payment_id2 = client.process_payment(&transaction_contract, &2);
    client.update_payment_status(&payment_id2, &PaymentStatus::Failed);
    let payment2 = client.get_payment(&payment_id2);
    assert_eq!(payment2.status, PaymentStatus::Failed);

    // Test transition from Processing to Completed (should work)
    let payment_id3 = client.process_payment(&transaction_contract, &3);
    client.update_payment_status(&payment_id3, &PaymentStatus::Processing);
    client.update_payment_status(&payment_id3, &PaymentStatus::Completed);
    let payment3 = client.get_payment(&payment_id3);
    assert_eq!(payment3.status, PaymentStatus::Completed);
}
