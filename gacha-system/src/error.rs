use std::error::Error;
use std::fmt::{Display, Formatter};

pub(crate) type Result<T> = std::result::Result<T, GachaError>;

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum GachaError {
    InvalidRarity(String),
    RarityWithNoData(String),
}

impl Display for GachaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use GachaError::*;
        let msg = match self {
            InvalidRarity(rty) => format!("\"{rty}\" is not a valid rarity in gacha pool"),
            RarityWithNoData(rty) => format!("gacha pool for rarity \"{rty}\" has no data"),
        };
        f.write_str(&msg)
    }
}

impl Error for GachaError {}
