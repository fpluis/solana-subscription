use solana_program::{hash::Hash, program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};
use spl_subscription::{
    instruction,
    processor::{PaySubscriptionArgs, CreateSubscriptionArgs, WithdrawFundsArgs},
};

pub async fn get_account(banks_client: &mut BanksClient, pubkey: &Pubkey) -> Account {
    banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> Result<(Keypair, Keypair), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let pool_mint = Keypair::new();
    let manager = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &pool_mint.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &pool_mint.pubkey(),
                &manager.pubkey(),
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, &pool_mint], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok((pool_mint, manager))
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    pool_mint: &Pubkey,
    manager: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                pool_mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn get_token_balance(banks_client: &mut BanksClient, token: &Pubkey) -> u64 {
    let token_account = banks_client.get_account(*token).await.unwrap().unwrap();
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(token_account.data.as_slice()).unwrap();
    account_info.amount
}

pub async fn get_token_supply(banks_client: &mut BanksClient, mint: &Pubkey) -> u64 {
    let mint_account = banks_client.get_account(*mint).await.unwrap().unwrap();
    let account_info =
        spl_token::state::Mint::unpack_from_slice(mint_account.data.as_slice()).unwrap();
    account_info.supply
}

pub async fn create_subscription(
    banks_client: &mut BanksClient,
    program_id: &Pubkey,
    payer: &Keypair,
    owner_addresses: Vec<Pubkey>,
    owner_shares: Vec<u8>,
    recent_blockhash: &Hash,
    resource: &Pubkey,
    mint_keypair: &Pubkey,
    price: &u64,
    period_duration: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::create_subscription_instruction(
            *program_id,
            payer.pubkey(),
            CreateSubscriptionArgs {
                owner_addresses: owner_addresses,
                owner_shares: owner_shares,
                token_mint: *mint_keypair,
                resource: *resource,
                price: *price,
                period_duration,
            },
        )],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn approve(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    payer: &Keypair,
    transfer_authority: &Pubkey,
    spl_wallet: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::approve(
            &spl_token::id(),
            &spl_wallet.pubkey(),
            transfer_authority,
            &payer.pubkey(),
            &[&payer.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn pay_subscription(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    payer: &Keypair,
    payer_token: &Keypair,
    subscription_funds_token: &Keypair,
    transfer_authority: &Keypair,
    resource: &Pubkey,
    mint: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::pay_subscription_instruction(
            *program_id,
            payer.pubkey(),       // Wallet used to identify bidder
            payer_token.pubkey(), // SPL Token Account (Source)
            subscription_funds_token.pubkey(), // SPL token account (Destination)
            *mint,                       // Token Mint
            transfer_authority.pubkey(), // Approved to Move Tokens
            PaySubscriptionArgs {
                resource: *resource,
            },
        )],
        Some(&payer.pubkey()),
        &[payer, transfer_authority, payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn withdraw_funds(
    banks_client: &mut BanksClient,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    payer: &Keypair,
    withdrawer_token: &Keypair,
    subscription_funds_token: &Keypair,
    amount: &u64,
    resource: &Pubkey,
    mint: &Pubkey,
) -> Result<(), TransportError> {
    let transaction = Transaction::new_signed_with_payer(
        &[instruction::withdraw_funds_instruction(
            *program_id,
            payer.pubkey(),
            withdrawer_token.pubkey(),
            withdrawer_token.pubkey(),
            subscription_funds_token.pubkey(),
            *mint,
            WithdrawFundsArgs {
                resource: *resource,
                amount: *amount,
            },
        )],
        Some(&payer.pubkey()),
        &[payer, withdrawer_token],
        *recent_blockhash,
    );
    println!("+ Created the tx for withdraw_funds");
    let client_result = banks_client.process_transaction(transaction).await?;
    println!("Client result: {:?}", client_result);
    Ok(())
}
