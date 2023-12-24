//! Besides regular commands and control commands there is also direct
//! memory access using datablocks.
//! Some vital parameters (e.g programmed battery capacity) are available
//! exclusively through this interface.

use crate::{Bq27xx, ChipError};
use embedded_hal_async::{delay, i2c};

const MEMBLOCK_SIZE: usize = 32;

/// All memory locations are divided into subclasses
pub mod memory_subclass {
    #![allow(dead_code)]
    pub const SAFETY: u8 = 2;
    pub const CHARGE_TERMINATION: u8 = 36;
    pub const DISCHARGE: u8 = 49;
    pub const REGISTERS: u8 = 64;
    pub const IT_CFG: u8 = 80;
    pub const CURRENT_THRESHOLDS: u8 = 81;
    pub const STATE: u8 = 82;
    pub const RA0_RAM: u8 = 89;
    pub const CHEM_DATA: u8 = 109;
    pub const DATA: u8 = 104;
    pub const CC_CAL: u8 = 105;
    pub const CURRENT: u8 = 107;
    pub const CODES: u8 = 112;
}

/// TODO: move this to the Command enums
pub mod def {
    pub const DATA_CLASS: u8 = 0x3E;
    pub const DATA_BLOCK: u8 = 0x3F;
    pub const BLOCK_DATA_START: u8 = 0x40;
    pub const BLOCK_DATA_CHECKSUM: u8 = 0x60;
    pub const BLOCK_DATA_CONTROL: u8 = 0x61;
}

pub struct MemoryBlock {
    pub raw: [u8; MEMBLOCK_SIZE],
}

impl MemoryBlock {
    /// Simple checksum used by the gauge
    fn checksum(&self) -> u8 {
        let mut csum: u8 = 0;

        for b in self.raw.iter() {
            csum = csum.wrapping_add(*b);
        }

        255 - csum
    }

    fn new() -> Self {
        Self {
            raw: [0; MEMBLOCK_SIZE],
        }
    }
}

impl<I, D, E> Bq27xx<I, D>
where
    D: delay::DelayNs,
    I: i2c::I2c<Error = E>,
{
    /// Read the selected block checksum from the gauge
    async fn read_checksum(&mut self) -> Result<u8, ChipError<E>> {
        let mut checksum = [0];

        self.i2c
            .write_read(self.addr, &[def::BLOCK_DATA_CHECKSUM], &mut checksum)
            .await?;

        Ok(checksum[0])
    }

    /// Prepares for the read or write operation
    async fn memblock_prepare_op(&mut self, class: u8, block: u8) -> Result<(), ChipError<E>> {
        self.i2c
            .write(self.addr, &[def::BLOCK_DATA_CONTROL, 0])
            .await?;

        self.i2c.write(self.addr, &[def::DATA_CLASS, class]).await?;
        self.i2c.write(self.addr, &[def::DATA_BLOCK, block]).await?;

        self.delay.delay_ms(5).await;

        Ok(())
    }

    /// Reads the entire memory block
    pub async fn memblock_read(&mut self, class: u8, at: u8) -> Result<MemoryBlock, ChipError<E>> {
        self.memblock_prepare_op(class, at).await?;

        let mut block = MemoryBlock::new();
        let checksum = self.read_checksum().await?;

        self.i2c
            .write_read(self.addr, &[def::BLOCK_DATA_START], &mut block.raw)
            .await?;

        if checksum != block.checksum() {
            Err(ChipError::Checksum)
        } else {
            Ok(block)
        }
    }

    /// Writes the entire memory block
    pub async fn memblock_write(
        &mut self,
        class: u8,
        at: u8,
        block: MemoryBlock,
    ) -> Result<(), ChipError<E>> {
        self.mode_cfgupdate().await?;
        self.memblock_prepare_op(class, at).await?;

        for (i, ptr) in block.raw.iter().enumerate() {
            self.i2c
                .write(self.addr, &[def::BLOCK_DATA_START + i as u8, *ptr])
                .await?;
        }

        // Propose our checksum to the chip
        let checksum = block.checksum();
        self.i2c
            .write(self.addr, &[def::BLOCK_DATA_CHECKSUM, checksum])
            .await?;

        // And let it think for a bit...
        self.delay.delay_ms(5).await;

        // Now we can read the checksum one more time to ensure that the chip got it right
        self.memblock_prepare_op(class, at).await?;

        let chip_checksum = self.read_checksum().await?;
        self.soft_reset().await?;

        if checksum == chip_checksum {
            Ok(())
        } else {
            Err(ChipError::Checksum)
        }
    }
}
