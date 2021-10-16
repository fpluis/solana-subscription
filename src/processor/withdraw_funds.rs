//! Claim bid winnings into a target SPL account, only the authorised key can do this, though the
//! target can be any SPL account.

use crate::{
    errors::SubscriptionError,
    processor::SubscriptionData,
    utils::{
        assert_derivation, assert_initialized, assert_owned_by, assert_signer,
        assert_token_program_matches_package, create_or_allocate_account_raw, spl_token_transfer,
        TokenTransferParams,
    },
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
    },
    spl_token::state::Account,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct WithdrawFundsArgs {
    pub resource: Pubkey,
    pub amount: u64,
}

struct Accounts<'a, 'b: 'a> {
    payer: &'a AccountInfo<'b>,
    withdrawer: &'a AccountInfo<'b>,
    withdrawer_token: &'a AccountInfo<'b>,
    subscription_funds_token: &'a AccountInfo<'b>,
    subscription: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        payer: next_account_info(account_iter)?,
        withdrawer: next_account_info(account_iter)?,
        withdrawer_token: next_account_info(account_iter)?,
        subscription_funds_token: next_account_info(account_iter)?,
        subscription: next_account_info(account_iter)?,
        mint: next_account_info(account_iter)?,
        token_program: next_account_info(account_iter)?,
    };

    assert_owned_by(accounts.subscription, program_id)?;
    assert_owned_by(accounts.mint, &spl_token::id())?;
    msg!(
        "Account owners: withdrawer_token ({:?}), funds ({:?})",
        accounts.withdrawer_token.owner,
        accounts.subscription_funds_token.owner
    );
    assert_owned_by(accounts.withdrawer_token, &spl_token::id())?;
    assert_owned_by(accounts.subscription_funds_token, &spl_token::id())?;
    assert_signer(accounts.payer)?;
    assert_signer(accounts.withdrawer)?;
    assert_token_program_matches_package(accounts.token_program)?;

    if *accounts.token_program.key != spl_token::id() {
        return Err(SubscriptionError::InvalidTokenProgram.into());
    }

    Ok(accounts)
}

pub fn withdraw_funds(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: WithdrawFundsArgs,
) -> ProgramResult {
    msg!("+ Processing WithdrawFunds, amount {}", args.amount);
    let accounts = parse_accounts(program_id, accounts)?;

    // The account within the pot must be owned by us.
    let actual_account: Account = assert_initialized(accounts.subscription_funds_token)?;
    if actual_account.owner != *accounts.subscription.key {
        return Err(SubscriptionError::FundsTokenAccountOwnerMismatch.into());
    }

    // Derive and load Subscription.
    let subscription_bump = assert_derivation(
        program_id,
        accounts.subscription,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.to_bytes(),
        ],
    )?;

    let subscription_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &args.resource.to_bytes(),
        &[subscription_bump],
    ];

    // Load the subscription and verify this bid is valid.
    let mut subscription = SubscriptionData::from_account_info(accounts.subscription)?;

    // The mint provided in this claim must match the one the subscription was initialized with.
    if subscription.token_mint != *accounts.mint.key {
        return Err(SubscriptionError::IncorrectMint.into());
    }

    let owner_index_option = subscription
        .owner_addresses
        .iter()
        .position(|address| address.as_ref() == accounts.withdrawer.key.as_ref());
    if owner_index_option == None {
        return Err(SubscriptionError::WithdrawerIsNotAnOwner.into());
    }
    let owner_index = owner_index_option.unwrap();
    msg!("Owner index: {}", owner_index);

    let owner_share = subscription.owner_shares.get(owner_index).unwrap();
    let share = f32::from(*owner_share);
    msg!("Owner share: {}", share);
    let percent = share / 100.0;
    msg!("Percent: {}", percent);

    let current_withdrawn = subscription.withdrawn_amounts.get(owner_index).unwrap();
    msg!("Current withdrawn: {}", current_withdrawn);
    let max_absolute_share = subscription.total_paid as f32 * percent;
    msg!("Max abs share: {}", max_absolute_share);
    let max_to_withdraw = max_absolute_share as u64 - current_withdrawn;
    msg!("Max to withdraw: {}", max_to_withdraw);
    if args.amount > max_to_withdraw {
        return Err(SubscriptionError::WithdrawalOverMaxAllowed.into());
    }

    msg!("SPL transfer with seeds {:?}", subscription_seeds);
    // Transfer requested amount to the owner.
    spl_token_transfer(TokenTransferParams {
        source: accounts.subscription_funds_token.clone(),
        destination: accounts.withdrawer_token.clone(),
        authority: accounts.subscription.clone(),
        authority_signer_seeds: subscription_seeds,
        token_program: accounts.token_program.clone(),
        amount: args.amount,
    })?;

    subscription.withdrawn_amounts[owner_index] += args.amount;
    subscription.serialize(&mut *accounts.subscription.data.borrow_mut())?;

    Ok(())
}
