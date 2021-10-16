use crate::{PREFIX};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

pub use crate::processor::{
    create_subscription::CreateSubscriptionArgs, pay_subscription::PaySubscriptionArgs,
    withdraw_funds::WithdrawFundsArgs,
};

#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum SubscriptionInstruction {
    /// Create a new subscription account bound to a resource
    ///   0. `[signer]` The account creating the subscription, which is authorised to make changes.
    ///   1. `[writable]` Uninitialized subscription account.
    ///   2. `[]` Rent sysvar
    ///   3. `[]` System account
    CreateSubscription(CreateSubscriptionArgs),

    /// Move SPL tokens from winning bid to the destination account.
    ///   0. `[writable]` The withdrawer's account as it appears in the list of owners.
    ///   1. `[signer]` The withdrawer's token account where the funds will be deposited.
    ///   2. `[writable]` The subscription funds token account
    ///   3. `[signer]` The authority on the subscription
    ///   4. `[]` The subscription
    ///   5. `[]` Token mint of the subscription
    ///   6. `[]` Token program
    WithdrawFunds(WithdrawFundsArgs),

    /// Update the authority for a subscription account.
    // SetAuthority,

    /// Add funds to a subscription.
    ///   0. `[signer]` The payer's primary account, for PDA calculation/transit auth.
    ///   1. `[writable]` The payer's token account
    ///   2. `[writable]` The subscription funds token account, where the tokens will be subscriptioned.
    ///   3. `[writable]` The pot SPL account,
    ///   4. `[writable]` The subscription account, storing information about the owners and the amounts they have withdrawn.
    ///   5. `[writable]` Token mint, for transfer instructions and verification.
    ///   6. `[signer]` Transfer authority, for moving tokens into the bid pot.
    ///   7. `[]` Rent sysvar
    ///   8. `[]` System program
    ///   9. `[]` SPL Token Program
    PaySubscription(PaySubscriptionArgs),
}

/// Creates an CreateSubscription instruction.
pub fn create_subscription_instruction(
    program_id: Pubkey,
    owner_pubkey: Pubkey,
    args: CreateSubscriptionArgs,
) -> Instruction {
    let seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (subscription_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
    println!(
        "@ Seeds during INSTRUCTION:create_subscription instruction: {:?}; pubkey: {}",
        seeds, subscription_pubkey
    );

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(owner_pubkey, true),
            AccountMeta::new(subscription_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: SubscriptionInstruction::CreateSubscription(args)
            .try_to_vec()
            .unwrap(),
    }
}

// /// Creates an SetAuthority instruction.
// pub fn set_authority_instruction(
//     program_id: Pubkey,
//     resource: Pubkey,
//     authority: Pubkey,
//     new_authority: Pubkey,
// ) -> Instruction {
//     let seeds = &[PREFIX.as_bytes(), program_id.as_ref(), resource.as_ref()];
//     let (subscription_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
//     Instruction {
//         program_id,
//         accounts: vec![
//             AccountMeta::new(subscription_pubkey, false),
//             AccountMeta::new_readonly(authority, true),
//             AccountMeta::new_readonly(new_authority, false),
//         ],
//         data: SubscriptionInstruction::SetAuthority.try_to_vec().unwrap(),
//     }
// }

/// Creates an PaySubscription instruction.
pub fn pay_subscription_instruction(
    program_id: Pubkey,
    payer_pubkey: Pubkey,
    payer_token_pubkey: Pubkey,
    subscription_funds_token_pubkey: Pubkey,
    token_mint_pubkey: Pubkey,
    transfer_authority: Pubkey,
    args: PaySubscriptionArgs,
) -> Instruction {
    // Derive Subscription Key
    let seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (subscription_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
    println!(
        "@ Seeds during INSTRUCTION:pay instruction: {:?}; pubkey: {}",
        seeds, subscription_pubkey
    );

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pubkey, true),
            AccountMeta::new(payer_token_pubkey, false),
            AccountMeta::new(subscription_funds_token_pubkey, false),
            AccountMeta::new(subscription_pubkey, false),
            AccountMeta::new(token_mint_pubkey, false),
            AccountMeta::new_readonly(transfer_authority, true),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: SubscriptionInstruction::PaySubscription(args)
            .try_to_vec()
            .unwrap(),
    }
}

pub fn withdraw_funds_instruction(
    program_id: Pubkey,
    payer_pubkey: Pubkey,
    withdrawer_pubkey: Pubkey,
    withdrawer_token_pubkey: Pubkey,
    subscription_funds_token: Pubkey,
    token_mint_pubkey: Pubkey,
    args: WithdrawFundsArgs,
) -> Instruction {
    // Derive Subscription Key
    let seeds = &[
        PREFIX.as_bytes(),
        program_id.as_ref(),
        args.resource.as_ref(),
    ];
    let (subscription_pubkey, _) = Pubkey::find_program_address(seeds, &program_id);
    println!(
        "@ Seeds during INSTRUCTION:withdraw_funds instruction: {:?}; pubkey: {}",
        seeds, subscription_pubkey
    );

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer_pubkey, true),
            AccountMeta::new(withdrawer_pubkey, true),
            AccountMeta::new(withdrawer_token_pubkey, false),
            AccountMeta::new(subscription_funds_token, false),
            AccountMeta::new(subscription_pubkey, false),
            AccountMeta::new_readonly(token_mint_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: SubscriptionInstruction::WithdrawFunds(args)
            .try_to_vec()
            .unwrap(),
    }
}
