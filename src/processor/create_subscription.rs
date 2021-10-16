use mem::size_of;

use crate::{
    errors::SubscriptionError,
    processor::{SubscriptionData, MAX_SUBSCRIPTION_SIZE, MAX_OWNER_LIMIT},
    utils::{assert_derivation, assert_owned_by, create_or_allocate_account_raw},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::UnixTimestamp,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::mem,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CreateSubscriptionArgs {
    // Subscription co-owner addresses
    pub owner_addresses: Vec<Pubkey>,
    // Subscription co-owner share percentages
    pub owner_shares: Vec<u8>,
    // Token mint for the SPL token being used to pay
    pub token_mint: Pubkey,
    // The resource associated to this subscription
    pub resource: Pubkey,
    // The price of each period extension
    pub price: u64,
    // The duration of each period in seconds
    pub period_duration: u64,
}

struct Accounts<'a, 'b: 'a> {
    subscription: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    system: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        payer: next_account_info(account_iter)?,
        subscription: next_account_info(account_iter)?,
        rent: next_account_info(account_iter)?,
        system: next_account_info(account_iter)?,
    };
    Ok(accounts)
}

pub fn create_subscription(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateSubscriptionArgs,
) -> ProgramResult {
    msg!("+ Processing CreateSubscription");
    let accounts = parse_accounts(program_id, accounts)?;

    let subscription_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &args.resource.to_bytes(),
    ];
    msg!(
        "@ Seeds during PROCESSOR:create_subscription instruction: {:?}",
        subscription_path
    );

    // Derive the address we'll store the subscription in, and confirm it matches what we expected the
    // user to provide.
    let (subscription_key, bump) = Pubkey::find_program_address(&subscription_path, program_id);
    if subscription_key != *accounts.subscription.key {
        return Err(SubscriptionError::InvalidSubscriptionAccount.into());
    }

    if args.owner_addresses.len() > MAX_OWNER_LIMIT {
        return Err(SubscriptionError::MaxOwnersExceeded.into());
    }

    if args.owner_shares.len() != args.owner_shares.len() {
        return Err(SubscriptionError::OwnerAddressesToSharesMismatch.into());
    }

    // The data must be large enough to hold at least:
    // - Each owner
    // - Each owned amount (u64 = 8 bytes)
    // - The base subscription data
    // let subscription_size = OWNER_SIZE * args.owner_addresses.len()
    //     + 8 * args.owner_addresses.len()
    //     + BASE_SUBSCRIPTION_DATA_SIZE;

    // Create subscription account with enough space
    create_or_allocate_account_raw(
        *program_id,
        accounts.subscription,
        accounts.rent,
        accounts.system,
        accounts.payer,
        MAX_SUBSCRIPTION_SIZE,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.to_bytes(),
            &[bump],
        ],
    )?;

    // Configure Subscription
    SubscriptionData {
        token_mint: args.token_mint,
        withdrawn_amounts: vec![0; args.owner_addresses.len()],
        owner_addresses: args.owner_addresses,
        owner_shares: args.owner_shares,
        total_paid: 0,
        price: args.price,
        period_duration: args.period_duration,
        paid_until: 0,
    }
    .serialize(&mut *accounts.subscription.data.borrow_mut())?;

    Ok(())
}
