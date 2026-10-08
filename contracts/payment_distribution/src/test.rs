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
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Batch process 3 payments with different recipients
    let mut payments = Vec::new(&env);
    payments.push_back((1u64, recipient1.clone(), 100_000i128));
    payments.push_back((2u64, recipient2.clone(), 200_000i128));
    payments.push_back((3u64, recipient3.clone(), 300_000i128));

    let payment_ids = client.batch_process(&payments);

    assert_eq!(payment_ids.len(), 3);
    assert_eq!(client.get_payment_count(), 3);

    // Verify all payments were created with correct data
    for i in 0..payment_ids.len() {
        if let Some(payment_id) = payment_ids.get(i) {
            let payment = client.get_payment(&payment_id);
            assert_eq!(payment.id, payment_id);
            assert_eq!(payment.status, PaymentStatus::Pending);
        }
    }

    // Verify each payment has correct recipient and amount
    let payment1 = client.get_payment(&1);
    assert_eq!(payment1.recipient, recipient1);
    assert_eq!(payment1.amount, 100_000);

    let payment2 = client.get_payment(&2);
    assert_eq!(payment2.recipient, recipient2);
    assert_eq!(payment2.amount, 200_000);

    let payment3 = client.get_payment(&3);
    assert_eq!(payment3.recipient, recipient3);
    assert_eq!(payment3.amount, 300_000);
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
fn test_batch_process_gas_optimization() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Test batch processing with larger batch size
    let mut payments = Vec::new(&env);
    for i in 1..=10 {
        let recipient = Address::generate(&env);
        payments.push_back((i, recipient, 50_000i128 * i as i128));
    }

    // Record budget before batch operation
    env.budget().reset_unlimited();

    let payment_ids = client.batch_process(&payments);

    // Verify all payments processed correctly
    assert_eq!(payment_ids.len(), 10);
    assert_eq!(client.get_payment_count(), 10);

    // Verify payment data integrity
    for i in 0..payment_ids.len() {
        if let Some(payment_id) = payment_ids.get(i) {
            let payment = client.get_payment(&payment_id);
            assert_eq!(payment.status, PaymentStatus::Pending);
            assert_eq!(payment.transaction_id, payment_id);
        }
    }
}

#[test]
fn test_batch_vs_individual_processing() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);

    // Test individual processing
    let (_, client1) = create_contract(&env);
    client1.initialize(&admin, &token_contract);

    env.budget().reset_unlimited();
    
    // Process 5 payments individually
    for i in 1..=5 {
        let recipient = Address::generate(&env);
        client1.process_payment(&(i as u64), &recipient, &100_000i128);
    }

    // Test batch processing
    let (_, client2) = create_contract(&env);
    client2.initialize(&admin, &token_contract);

    env.budget().reset_unlimited();

    // Process 5 payments in batch
    let mut payments = Vec::new(&env);
    for i in 1..=5 {
        let recipient = Address::generate(&env);
        payments.push_back((i as u64, recipient, 100_000i128));
    }

    client2.batch_process(&payments);

    // Both should produce same result
    assert_eq!(client1.get_payment_count(), 5);
    assert_eq!(client2.get_payment_count(), 5);
}

#[test]
fn test_batch_process_empty_list() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process empty batch
    let payments = Vec::new(&env);
    let payment_ids = client.batch_process(&payments);

    assert_eq!(payment_ids.len(), 0);
    assert_eq!(client.get_payment_count(), 0);
}

#[test]
fn test_batch_process_single_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Process single payment via batch
    let mut payments = Vec::new(&env);
    payments.push_back((1u64, recipient.clone(), 100_000i128));

    let payment_ids = client.batch_process(&payments);

    assert_eq!(payment_ids.len(), 1);
    assert_eq!(client.get_payment_count(), 1);

    let payment = client.get_payment(&1);
    assert_eq!(payment.recipient, recipient);
    assert_eq!(payment.amount, 100_000);
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_batch_process_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Try to batch process with negative amount
    let mut payments = Vec::new(&env);
    payments.push_back((1u64, recipient.clone(), -100_000i128));

    client.batch_process(&payments);
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_batch_process_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Try to batch process with zero amount
    let mut payments = Vec::new(&env);
    payments.push_back((1u64, recipient.clone(), 0i128));

    client.batch_process(&payments);
}

#[test]
fn test_batch_process_same_recipient_multiple_times() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Batch process multiple payments to same recipient
    let mut payments = Vec::new(&env);
    payments.push_back((1u64, recipient.clone(), 100_000i128));
    payments.push_back((2u64, recipient.clone(), 200_000i128));
    payments.push_back((3u64, recipient.clone(), 300_000i128));

    let payment_ids = client.batch_process(&payments);

    assert_eq!(payment_ids.len(), 3);
    assert_eq!(client.get_payment_count(), 3);

    // Verify recipient has all payments indexed
    let recipient_payments = client.get_recipient_payments(&recipient, &10);
    assert_eq!(recipient_payments.len(), 3);

    // Verify total statistics for recipient
    let (total_payments, total_amount, completed, pending) = 
        client.get_recipient_statistics(&recipient);
    assert_eq!(total_payments, 3);
    assert_eq!(total_amount, 600_000);
    assert_eq!(pending, 3);
    assert_eq!(completed, 0);
}

#[test]
fn test_batch_process_payment_counter_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // First batch
    let mut payments1 = Vec::new(&env);
    for i in 1..=3 {
        let recipient = Address::generate(&env);
        payments1.push_back((i, recipient, 100_000i128));
    }
    let ids1 = client.batch_process(&payments1);

    // Second batch
    let mut payments2 = Vec::new(&env);
    for i in 4..=7 {
        let recipient = Address::generate(&env);
        payments2.push_back((i, recipient, 200_000i128));
    }
    let ids2 = client.batch_process(&payments2);

    // Verify payment IDs are sequential across batches
    assert_eq!(ids1.get(0).unwrap(), 1);
    assert_eq!(ids1.get(2).unwrap(), 3);
    assert_eq!(ids2.get(0).unwrap(), 4);
    assert_eq!(ids2.get(3).unwrap(), 7);

    assert_eq!(client.get_payment_count(), 7);
}

#[test]
fn test_batch_process_maintains_timestamp_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 5000);

    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let (_, client) = create_contract(&env);

    client.initialize(&admin, &token_contract);

    // Batch process multiple payments
    let mut payments = Vec::new(&env);
    for i in 1..=5 {
        let recipient = Address::generate(&env);
        payments.push_back((i, recipient, 100_000i128));
    }

    let payment_ids = client.batch_process(&payments);

    // All payments in batch should have same timestamp
    let first_payment = client.get_payment(&payment_ids.get(0).unwrap());
    for i in 1..payment_ids.len() {
        let payment = client.get_payment(&payment_ids.get(i).unwrap());
        assert_eq!(payment.created_at, first_payment.created_at);
        assert_eq!(payment.created_at, 5000);
    }
}
