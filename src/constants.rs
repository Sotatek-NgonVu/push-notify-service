use serde::{Deserialize, Serialize};
use strum::{EnumString};
use strum_macros::VariantNames;

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    EnumString,
    VariantNames,
    strum_macros::Display,
    PartialEq,
    Eq,
    Hash,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TradingType {
    Buy,
    Sell,
    Add,
    Remove,
}