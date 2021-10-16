use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the Subscription program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SubscriptionError {
    /// Account does not have correct owner
    #[error("Account does not have correct owner")]
    IncorrectOwner,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Subscription account specified is invalid.
    #[error("Subscription account specified is invalid.")]
    InvalidSubscriptionAccount,

    /// Balance too low to make bid.
    #[error("Balance too low to make bid.")]
    BalanceTooLow,

    /// Failed to derive an account from seeds.
    #[error("Failed to derive an account from seeds.")]
    DerivedKeyInvalid,

    /// Token transfer failed
    #[error("Token transfer failed")]
    TokenTransferFailed,

    /// Invalid authority
    #[error("Invalid authority")]
    InvalidAuthority,

    /// Authority not signer
    #[error("Authority not signer")]
    AuthorityNotSigner,

    /// Numerical overflow
    #[error("Numerical overflow")]
    NumericalOverflowError,

    /// Uninitialized
    #[error("Uninitialized")]
    Uninitialized,

    /// Existing Bid is already active.
    #[error("Existing Bid is already active.")]
    BidAlreadyActive,

    /// Incorrect mint specified, must match subscription.
    #[error("Incorrect mint specified, must match subscription.")]
    IncorrectMint,

    /// Must reveal price when ending a blinded subscription.
    #[error("Must reveal price when ending a blinded subscription.")]
    MustReveal,

    /// The revealing hash is invalid.
    #[error("The revealing hash is invalid.")]
    InvalidReveal,

    /// The pot for this bid is already empty.
    #[error("The pot for this bid is already empty.")]
    BidderPotEmpty,

    /// This is not a valid token program
    #[error(" This is not a valid token program")]
    InvalidTokenProgram,

    /// Accept payment delegate should be none
    #[error("Accept payment delegate should be none")]
    DelegateShouldBeNone,

    /// Accept payment close authority should be none
    #[error("Accept payment close authority should be none")]
    CloseAuthorityShouldBeNone,

    /// Data type mismatch
    #[error("Data type mismatch")]
    DataTypeMismatch,

    /// Bid must be multiple of tick size
    #[error("Bid must be multiple of tick size")]
    BidMustBeMultipleOfTickSize,

    /// During the gap window, gap between next lowest bid must be of a certain percentage
    #[error("During the gap window, gap between next lowest bid must be of a certain percentage")]
    GapBetweenBidsTooSmall,

    /// Gap tick size percentage must be between 0 and 100
    #[error("Gap tick size percentage must be between 0 and 100")]
    InvalidGapTickSizePercentage,

    /// There are more than 5 owners, which is the hard-limit for Metaplex
    #[error("There are more than 5 owners, which is the hard-limit for Metaplex.")]
    MaxOwnersExceeded,

    /// The number of addresses and shares are different but they must be the same
    #[error("The number of addresses and shares are different but they must be the same.")]
    OwnerAddressesToSharesMismatch,

    /// This subscription account does not own the actual token account
    #[error("This subscription account does not own the actual token account.")]
    FundsTokenAccountOwnerMismatch,

    /// There is no funds subscription account associated to this account
    #[error("There is no funds subscription account associated to this account.")]
    SubscriptionFundsAccountDoesNotExist,

    /// The withdrawer is not listed as an owner of this account.
    #[error("The withdrawer is not listed as an owner of this account.")]
    WithdrawerIsNotAnOwner,

    /// The withdrawal exceeds the amount that belongs to the co-owner according to their share.
    #[error("The withdrawal exceeds the amount that belongs to the co-owner according to their share.")]
    WithdrawalOverMaxAllowed
}

impl PrintProgramError for SubscriptionError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<SubscriptionError> for ProgramError {
    fn from(e: SubscriptionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for SubscriptionError {
    fn type_of() -> &'static str {
        "Vault Error"
    }
}
