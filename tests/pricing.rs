//! Which of Roblox's two pricing fields a write sends.
//!
//! Roblox accepts `isRegionalPricingEnabled` or `isManagedPricingEnabled`,
//! never both, and marks the regional one deprecated. These tests pin the
//! resolution from the two config keys to the one field, because getting it
//! wrong is silent: the request still succeeds, it just sets the other thing
//! or nothing at all.

use rbxsync::api::Pricing;

#[test]
fn an_unstated_config_sends_no_pricing_field() {
    // The case that matters most. Roblox enables managed pricing by itself on
    // passes, so writing either field here would turn that off on every sync,
    // for every resource, in a config that never mentioned pricing.
    assert_eq!(
        Pricing::from_config(false, None),
        Pricing::Untouched,
        "a config stating nothing must delegate, not assert a false"
    );
}

#[test]
fn managed_pricing_wins_and_the_deprecated_field_stays_out() {
    assert_eq!(
        Pricing::from_config(false, Some(true)),
        Pricing::Managed(true)
    );
    assert_eq!(
        Pricing::from_config(false, Some(false)),
        Pricing::Managed(false),
        "an explicit false is a real opt-out, not a silence"
    );
}

#[test]
fn an_explicit_regional_true_still_sends_the_deprecated_field() {
    // Deprecated is not removed. A config that already asked for it keeps
    // working exactly as it did.
    assert_eq!(Pricing::from_config(true, None), Pricing::Regional(true));
}

#[test]
fn managed_pricing_beats_regional_when_both_somehow_arrive() {
    // `Config::load` refuses this pair before any request is built, so this
    // only pins the fallback: whichever way it is reached, the field Roblox
    // still maintains is the one sent, and never both.
    assert_eq!(
        Pricing::from_config(true, Some(true)),
        Pricing::Managed(true)
    );
}
