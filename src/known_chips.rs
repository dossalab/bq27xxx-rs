//! This module is tested on BQ27427, but I suspect that the interface
//! and commands are pretty similar across all TI bridges. YMMV

use embedded_hal_async::{delay, i2c};

#[cfg(feature = "defmt")]
use defmt::{debug, info};

use crate::{memory, Bq27xx, ChipError};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChipType {
    BQ27421,
    BQ27426,
    BQ27427,
    Unknown,
}

impl<I, D, E> Bq27xx<I, D>
where
    D: delay::DelayNs,
    I: i2c::I2c<Error = E>,
{
    // This is documented on TI's engineering forum:
    // https://e2e.ti.com/support/power-management-group/power-management/f/power-management-forum/1215460/bq27427evm-misbehaving-stateofcharge
    // It's also possible to change that using generic memblock interface. This is just a bit faster and requires less i2c transactions
    pub(crate) async fn check_fix_bq27427_errata(&mut self) -> Result<(), ChipError<E>> {
        let mut buffer: [u8; 1] = [0];

        const SIGN_BIT: u8 = 1 << 7;
        const SIGN_BYTE_OFFSET: u8 = memory::def::BLOCK_DATA_START + 5;

        self.memblock_prepare_op(memory::memory_subclass::CC_CAL, 0)
            .await?;

        self.i2c
            .write_read(self.addr, &[SIGN_BYTE_OFFSET], &mut buffer)
            .await?;

        let cc_gain_sign_part = buffer[0];

        #[cfg(feature = "defmt")]
        debug!("BQ27427: CC gain sign byte is 0x{:02x}", cc_gain_sign_part);

        if cc_gain_sign_part & SIGN_BIT > 0 {
            #[cfg(feature = "defmt")]
            info!("BQ27427: applying the CC gain fix");

            let checksum = self.read_checksum().await?;

            self.mode_cfgupdate().await?;
            self.i2c
                .write(self.addr, &[SIGN_BYTE_OFFSET, cc_gain_sign_part ^ SIGN_BIT])
                .await?;

            self.i2c
                .write(
                    self.addr,
                    &[memory::def::BLOCK_DATA_CHECKSUM, checksum ^ SIGN_BIT],
                )
                .await?;

            self.soft_reset().await?;
        }

        Ok(())
    }
}
