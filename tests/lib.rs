#![allow(warnings)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program_test::*;
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    account::Account,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    log::sol_log,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
    transport::TransportError,
};
use spl_subscription::{
    instruction,
    processor::{process_instruction, CreateSubscriptionArgs, SubscriptionData},
    PREFIX,
};
use std::mem;

mod helpers;

async fn setup_subscription(
    shares: Vec<u8>,
    price: u64,
) -> (
    Pubkey,
    BanksClient,
    Vec<Keypair>,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Keypair,
    Hash,
    SubscriptionData,
) {
    // Create a program to attach accounts to.
    let program_id = Pubkey::new_rand();
    let mut program_test = ProgramTest::new(
        "spl_subscription",
        program_id,
        processor!(process_instruction),
    );
    
    // Start executing test.
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a Token mint to mint some test tokens with.
    let (mint_keypair, mint_manager) =
    helpers::create_mint(&mut banks_client, &payer, &recent_blockhash)
    .await
    .unwrap();
    
    // Derive Subscription PDA account for lookup.
    let resource = Pubkey::new_rand();
    println!("Program id: {:?}, resource: {:?}", program_id, resource);
    let seeds = &[
        PREFIX.as_bytes(),
        &program_id.as_ref(),
        resource.as_ref(),
    ];
    let (subscription_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);

    // PDA in the subscription for the Bidder to subscription their funds to.
    let subscription_token_account = Keypair::new();

    // Generate Subscription SPL account to transfer to.
    helpers::create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &subscription_token_account,
        &mint_keypair.pubkey(),
        &subscription_pubkey,
    )
    .await
    .unwrap();

    // Attach useful Accounts for testing.
    let mut owner_addresses = vec![];
    let mut owner_shares = vec![];
    let mut keypairs = vec![];
    for n in 0..shares.len() {
        println!("Create owner {:?}", n);
        // Bidder SPL Account, with Minted Tokens
        let keypair = Keypair::new();
        // Generate User SPL Wallet Account
        helpers::create_token_account(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &keypair,
            &mint_keypair.pubkey(),
            &payer.pubkey(),
        )
        .await
        .unwrap();

        // Mint Tokens
        helpers::mint_tokens(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            &mint_keypair.pubkey(),
            &keypair.pubkey(),
            &mint_manager,
            10_000_000,
        )
        .await
        .unwrap();

        owner_addresses.push(keypair.pubkey());
        owner_shares.push(shares[n]);
        keypairs.push(keypair);
    }

    println!("Create Subscription");
    // Run Create Subscription instruction.
    let err = helpers::create_subscription(
        &mut banks_client,
        &program_id,
        &payer,
        owner_addresses,
        owner_shares,
        &recent_blockhash,
        &resource,
        &mint_keypair.pubkey(),
        &price,
        1000,
    )
    .await
    .unwrap();

    println!("Verify Subscription");
    // Verify Subscription was created as expected.
    let subscription: SubscriptionData = try_from_slice_unchecked(
        &banks_client
            .get_account(subscription_pubkey)
            .await
            .expect("get_account")
            .expect("account not found")
            .data,
    )
    .unwrap();

    println!("Run asserts");
    assert_eq!(subscription.token_mint, mint_keypair.pubkey());
    assert_eq!(subscription.total_paid, 0);
    // assert_eq!(subscription.authority, payer.pubkey());

    return (
        program_id,
        banks_client,
        keypairs,
        payer,
        resource,
        mint_keypair.pubkey(),
        mint_manager.pubkey(),
        subscription_pubkey,
        subscription_token_account,
        recent_blockhash,
        subscription,
    );
}

// #[cfg(feature = "test-bpf")]
// #[tokio::test]
async fn run_tests_single_owner() {
    println!("Test with a single-owner subscription");
    let subscription_price = 500;
    let (
        program_id,
        mut banks_client,
        keypairs,
        payer,
        resource,
        mint,
        mint_authority,
        subscription_pubkey,
        subscription_token_account,
        recent_blockhash,
        subscription,
    ) = setup_subscription(vec![100], subscription_price).await;
    assert_eq!(subscription.withdrawn_amounts, vec!(0));

    let pre_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    let subscription_price = 500;
    let transfer_authority = Keypair::new();
    helpers::approve(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &transfer_authority.pubkey(),
        &keypairs[0],
        subscription_price,
    )
    .await
    .expect("approve");

    let result = helpers::pay_subscription(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[0],
        &subscription_token_account,
        &transfer_authority,
        &resource,
        &mint,
    )
    .await;
    println!("Add funds result: {:?}", result);

    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    assert_eq!(post_balance.0, pre_balance.0 - subscription_price);
    assert_eq!(post_balance.1, pre_balance.1 + subscription_price);

    println!("Withdraw {} funds from the account", subscription_price);
    let result = helpers::withdraw_funds(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[0],
        &subscription_token_account,
        &subscription_price as &u64,
        &resource,
        &mint,
    )
    .await;
    println!("Withdraw funds result: {:?}", result);
    let post_balance_2 = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    println!(
        "Funds withdrawn; post balance for owner: {}",
        post_balance.0
    );
    assert_eq!(post_balance_2.0, post_balance.0 + subscription_price);
    assert_eq!(post_balance_2.1, post_balance.1 - subscription_price);
}

async fn run_tests_multi_owner() {
    println!("Test with a multi-owner subscription");
    let subscription_price = 1000;
    let (
        program_id,
        mut banks_client,
        keypairs,
        payer,
        resource,
        mint,
        mint_authority,
        subscription_pubkey,
        subscription_token_account,
        recent_blockhash,
        subscription,
    ) = setup_subscription(vec![80, 20], subscription_price).await;
    assert_eq!(subscription.withdrawn_amounts, vec!(0, 0));

    let pre_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    let subscription_price = 1000;
    let transfer_authority = Keypair::new();
    helpers::approve(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &transfer_authority.pubkey(),
        &keypairs[0],
        subscription_price,
    )
    .await
    .expect("approve");

    let result = helpers::pay_subscription(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[0],
        &subscription_token_account,
        &transfer_authority,
        &resource,
        &mint,
    )
    .await;
    println!("Add first funds result: {:?}", result);

    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    assert_eq!(post_balance.0, pre_balance.0 - subscription_price);
    assert_eq!(post_balance.1, pre_balance.1 + subscription_price);

    let pre_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[1].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    let half_funds = 500;
    println!(
        "Try to withdraw {} funds from the account, which is over the allowed limit",
        half_funds
    );
    let result = helpers::withdraw_funds(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[1],
        &subscription_token_account,
        &half_funds,
        &resource,
        &mint,
    )
    .await;
    println!("Withdraw funds result: {:?}", result);
    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[1].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    println!(
        "Funds overdrawn; post balance for smaller co-owner: {}",
        post_balance.0
    );
    assert_eq!(post_balance.0, pre_balance.0);
    assert_eq!(post_balance.1, pre_balance.1);

    let allowed_amount = 100;
    println!("Withdraw {} funds from the account", allowed_amount);
    let result = helpers::withdraw_funds(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[1],
        &subscription_token_account,
        &allowed_amount as &u64,
        &resource,
        &mint,
    )
    .await;
    println!("Withdraw funds result: {:?}", result);
    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[1].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    println!(
        "Funds withdrawn; post balance for smaller co-owner: {}",
        post_balance.0
    );
    assert_eq!(post_balance.0, pre_balance.0 + allowed_amount);
    assert_eq!(post_balance.1, pre_balance.1 - allowed_amount);

    // Add funds again
    let pre_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    let subscription_price = 1000;
    let transfer_authority = Keypair::new();
    helpers::approve(
        &mut banks_client,
        &recent_blockhash,
        &payer,
        &transfer_authority.pubkey(),
        &keypairs[0],
        subscription_price,
    )
    .await
    .expect("approve");

    let result = helpers::pay_subscription(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[0],
        &subscription_token_account,
        &transfer_authority,
        &resource,
        &mint,
    )
    .await;
    println!("Add funds result: {:?}", result);

    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[0].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    assert_eq!(post_balance.0, pre_balance.0 - subscription_price);
    assert_eq!(post_balance.1, pre_balance.1 + subscription_price);

    let pre_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[1].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    // Retry withdrawing funds now that the limit has increased from 20% of 1000 (200) to 20% of 2000 (400)
    let allowed_amount = 300;
    println!(
        "Withdraw the remaining funds, {}, from the account",
        allowed_amount
    );
    let result = helpers::withdraw_funds(
        &mut banks_client,
        &recent_blockhash,
        &program_id,
        &payer,
        &keypairs[1],
        &subscription_token_account,
        &allowed_amount as &u64,
        &resource,
        &mint,
    )
    .await;
    println!("Withdraw funds result: {:?}", result);
    let post_balance = (
        helpers::get_token_balance(&mut banks_client, &keypairs[1].pubkey()).await,
        helpers::get_token_balance(&mut banks_client, &subscription_token_account.pubkey()).await,
    );

    println!(
        "Funds withdrawn; post balance for smaller co-owner: {}",
        post_balance.0
    );
    assert_eq!(post_balance.0, pre_balance.0 + allowed_amount);
    assert_eq!(post_balance.1, pre_balance.1 - allowed_amount);

    // Verify the final state is correct
    let subscription: SubscriptionData = try_from_slice_unchecked(
        &banks_client
            .get_account(subscription_pubkey)
            .await
            .expect("get_account")
            .expect("account not found")
            .data,
    )
    .unwrap();
    assert_eq!(subscription.withdrawn_amounts, vec!(0, 400));
    assert_eq!(subscription.total_paid, 2000);
}

#[cfg(feature = "test-bpf")]
#[tokio::test]
async fn run_tests() {
    run_tests_single_owner().await;
    run_tests_multi_owner().await;
}
