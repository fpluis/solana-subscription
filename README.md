---
title: Subscription Program
---

## Introduction

This program acts similarly to a deposit account where anyone can deposit funds, but only the owners can withdraw. The data held in a Subscription account is the following:

```rust
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
```

There are three simple instructions:
1. create_subscription: The owners create the subscription and pay rent for the account to stay open indefinitely and specify the subscription config: ```owners```, ```shares```, ```price``` (per period) and ```period_duration```.
2. pay_subscription: The payer has to transfer an amount superior to ```price``` tokens of the ```token_mint```. This will increase the subscription's ```paid_until``` by ```period_duration``` and the ```total_paid``` by ```price```.
3. withdraw_funds: One of the owners can withdraw up to ```total_paid``` * (their ```owner_share``` / 100) - their ```withdrawn_amount```. This will increase their ```withdrawn_amounts``` by the amount they withdraw.

Currently, subscriptions can be extended indefinitely before they expire. Some applications might not want to allow this behavior if, for example, the currency they use fluctuates too much in price and this property could be exploited. I plan to add a boolean option to disallow this behavior when creating the subscription.

Right now subscriptions support only one payment plan: single periods for a single price. In the future I would like to introduce different payment options, to model offers of the kind "3 months for the price of 2".