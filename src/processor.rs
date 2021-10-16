use crate::errors::SubscriptionError;
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, borsh::try_from_slice_unchecked, clock::UnixTimestamp,
    entrypoint::ProgramResult, hash::Hash, msg, program_error::ProgramError, pubkey::Pubkey,
};
use std::{cell::Ref, cmp, mem};

// Declare submodules, each contains a single handler for each instruction variant in the program.
pub mod withdraw_funds;
pub mod create_subscription;
pub mod pay_subscription;

// Re-export submodules handlers + associated types for other programs to consume.
pub use withdraw_funds::*;
pub use create_subscription::*;
pub use pay_subscription::*;
// pub use set_authority::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    use crate::instruction::SubscriptionInstruction;
    match SubscriptionInstruction::try_from_slice(input)? {
        SubscriptionInstruction::WithdrawFunds(args) => withdraw_funds(program_id, accounts, args),
        SubscriptionInstruction::CreateSubscription(args) => create_subscription(program_id, accounts, args),
        SubscriptionInstruction::PaySubscription(args) => pay_subscription(program_id, accounts, args),
        // SubscriptionInstruction::SetAuthority => set_authority(program_id, accounts),
    }
}

// #[repr(C)]
// #[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
// pub struct Owner {
//     pub address: Pubkey,
//     // In percentages, NOT basis points ;) Watch out!
//     pub share: u8,
// }

// 8 (Pubkey) + 1 (u8)
pub const OWNER_SIZE: usize = 8 + 1;

pub const MAX_OWNER_LIMIT: usize = 5;

pub const BASE_SUBSCRIPTION_DATA_SIZE: usize = 32 + 8 + 8 + 8;

// Base size + 5 addresses (PubKeys) + 5 shares (u8) + 5 withdrawn amounts (u64)
pub const MAX_SUBSCRIPTION_SIZE: usize = BASE_SUBSCRIPTION_DATA_SIZE + MAX_OWNER_LIMIT * 32 + MAX_OWNER_LIMIT * 1 + MAX_OWNER_LIMIT * 8;

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct SubscriptionData {
    /// Token mint for the SPL token being used to bid
    pub token_mint: Pubkey,
    // Subscription co-owner addresses
    pub owner_addresses: Vec<Pubkey>,
    // Subscription co-owner share percentages
    pub owner_shares: Vec<u8>,
    /// The time the last bid was placed, used to keep track of subscription timing.
    pub withdrawn_amounts: Vec<u64>,
    /// Slot time the subscription was officially ended by.
    pub total_paid: u64,
    // The price of each period extension
    pub price: u64,
    // The duration of each period in seconds
    pub period_duration: u64,
    // The UNIX timestamp when the subscription ends
    pub paid_until: UnixTimestamp,
}

impl SubscriptionData {
    // Cheap methods to get at SubscriptionData without supremely expensive borsh deserialization calls.

    pub fn get_token_mint(a: &AccountInfo) -> Pubkey {
        let data = a.data.borrow();
        let token_mint_data = array_ref![data, 32, 32];
        Pubkey::new_from_array(*token_mint_data)
    }

    pub fn from_account_info(a: &AccountInfo) -> Result<SubscriptionData, ProgramError> {
        let subscription: SubscriptionData = try_from_slice_unchecked(&a.data.borrow_mut())?;

        Ok(subscription)
    }

    pub fn add_funds(&mut self, amount: u64) -> ProgramResult {
        msg!("Adding funds {:?}", &amount.to_string());
        self.total_paid = self.total_paid + amount;
        Ok(())
    }
}
