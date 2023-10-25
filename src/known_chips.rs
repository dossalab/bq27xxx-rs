//! This module is tested on BQ27427, but I suspect that the interface
//! and commands are pretty similar across all TI bridges. YMMV

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChipType {
    BQ27421,
    BQ27426,
    BQ27427,
    Unknown,
}

impl From<u16> for ChipType {
    fn from(code: u16) -> Self {
        match code {
            0x421 => Self::BQ27421,
            0x426 => Self::BQ27426,
            0x427 => Self::BQ27427,
            _ => Self::Unknown,
        }
    }
}
