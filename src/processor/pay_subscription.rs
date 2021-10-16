use borsh::try_to_vec_with_schema;

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
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_option::COption,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        system_instruction::create_account,
        sysvar::{clock::Clock, Sysvar},
    },
    spl_token::state::Account,
    std::mem,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct PaySubscriptionArgs {
    /// Resource associated to this subscription (token mint in Metaplex).
    pub resource: Pubkey,
}

struct Accounts<'a, 'b: 'a> {
    payer: &'a AccountInfo<'b>,
    payer_token: &'a AccountInfo<'b>,
    subscription_funds_token: &'a AccountInfo<'b>,
    subscription: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    transfer_authority: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    clock_sysvar: &'a AccountInfo<'b>,
    system: &'a AccountInfo<'b>,
    token_program: &'a AccountInfo<'b>,
}

fn parse_accounts<'a, 'b: 'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
) -> Result<Accounts<'a, 'b>, ProgramError> {
    let account_iter = &mut accounts.iter();
    let accounts = Accounts {
        payer: next_account_info(account_iter)?,
        payer_token: next_account_info(account_iter)?,
        subscription_funds_token: next_account_info(account_iter)?,
        subscription: next_account_info(account_iter)?,
        mint: next_account_info(account_iter)?,
        transfer_authority: next_account_info(account_iter)?,
        rent: next_account_info(account_iter)?,
        clock_sysvar: next_account_info(account_iter)?,
        system: next_account_info(account_iter)?,
        token_program: next_account_info(account_iter)?,
    };

    assert_owned_by(accounts.subscription, program_id)?;
    assert_owned_by(accounts.payer_token, &spl_token::id())?;

    assert_owned_by(accounts.mint, &spl_token::id())?;
    assert_owned_by(accounts.subscription_funds_token, &spl_token::id())?;
    assert_signer(accounts.payer)?;
    assert_signer(accounts.transfer_authority)?;
    assert_token_program_matches_package(accounts.token_program)?;

    if *accounts.token_program.key != spl_token::id() {
        return Err(SubscriptionError::InvalidTokenProgram.into());
    }

    Ok(accounts)
}

#[allow(clippy::absurd_extreme_comparisons)]
pub fn pay_subscription<'r, 'b: 'r>(
    program_id: &Pubkey,
    accounts: &'r [AccountInfo<'b>],
    args: PaySubscriptionArgs,
) -> ProgramResult {
    msg!("+ Processing PaySubscription");
    let accounts = parse_accounts(program_id, accounts)?;

    // Load the subscription and verify this bid is valid.
    let mut subscription = SubscriptionData::from_account_info(accounts.subscription)?;

    // Check we own the account that contains the tokens
    let actual_account: Account = assert_initialized(accounts.subscription_funds_token)?;
    if actual_account.owner != *accounts.subscription.key {
        return Err(SubscriptionError::FundsTokenAccountOwnerMismatch.into());
    }

    if actual_account.delegate != COption::None {
        return Err(SubscriptionError::DelegateShouldBeNone.into());
    }

    if actual_account.close_authority != COption::None {
        return Err(SubscriptionError::CloseAuthorityShouldBeNone.into());
    }

    msg!(
        "+ Derive and load Subscription using seeds {:?}",
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.to_bytes(),
        ]
    );
    // Derive and load Subscription.
    // let subscription_bump = assert_derivation(
    //     program_id,
    //     accounts.subscription,
    //     &[
    //         PREFIX.as_bytes(),
    //         program_id.as_ref(),
    //         &args.resource.to_bytes(),
    //     ],
    // )?;
    let (_, subscription_bump) = Pubkey::find_program_address(
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            accounts.subscription.key.as_ref(),
        ],
        program_id,
    );

    msg!("+ Subscription bump: {:?}", subscription_bump);
    let authority_signer_seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        accounts.subscription.key.as_ref(),
        &[subscription_bump],
    ];

    msg!("+ About to check balance in account is enough");
    // Confirm payers SPL token balance is enough to pay the bid.
    let account: Account = Account::unpack_from_slice(&accounts.payer_token.data.borrow())?;
    msg!("+ Amount in account: {}", account.amount);
    if account.amount.saturating_sub(subscription.price) < 0 {
        msg!(
            "Amount in account is too small: {:?}, compared to subscription price {:?}",
            account.amount,
            subscription.price,
        );
        return Err(SubscriptionError::BalanceTooLow.into());
    }

    msg!("SPL transfer with seeds {:?}", authority_signer_seeds);
    // Transfer amount of SPL token to bid account.
    let err = spl_token_transfer(TokenTransferParams {
        source: accounts.payer_token.clone(),
        destination: accounts.subscription_funds_token.clone(),
        authority: accounts.transfer_authority.clone(),
        authority_signer_seeds,
        token_program: accounts.token_program.clone(),
        amount: subscription.price,
    })?;
    msg!("Result from transfer {:?}", err);

    // Serialize new Subscription State
    subscription.add_funds(subscription.price)?;
    let clock = Clock::from_account_info(accounts.clock_sysvar)?;

    msg!("Current clock timestamp {:?}", clock.unix_timestamp);
    if subscription.paid_until < clock.unix_timestamp {
        subscription.paid_until = clock.unix_timestamp + subscription.period_duration as i64;
    } else {
        subscription.paid_until = subscription.paid_until + subscription.period_duration as i64;
    }
    subscription.serialize(&mut *accounts.subscription.data.borrow_mut())?;
    Ok(())
}
