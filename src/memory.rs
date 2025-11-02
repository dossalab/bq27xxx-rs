//! Besides regular commands and control commands there is also direct
//! memory access using datablocks.
//! Some vital parameters (e.g programmed battery capacity) are available
//! exclusively through this interface.

use crate::{Bq27xx, ChipError};
use embedded_hal_async::{delay, i2c};

pub trait MemoryBlock {
    fn as_bytes(&self) -> &[u8];
    fn as_bytes_mut(&mut self) -> &mut [u8];
    fn get_class(&self) -> u8;

    fn write_u8(&mut self, at: usize, value: u8) {
        let mem = self.as_bytes_mut();
        mem[at] = value
    }

    fn read_u8(&self, at: usize) -> u8 {
        let mem = self.as_bytes();
        mem[at]
    }

    fn write_u16(&mut self, at: usize, value: u16) {
        let mem = self.as_bytes_mut();

        mem[at] = (value >> 8) as u8;
        mem[at + 1] = value as u8;
    }

    fn read_u16(&self, at: usize) -> u16 {
        let mem = self.as_bytes();
        ((mem[at] as u16) << 8) | mem[at + 1] as u16
    }

    fn checksum(&self) -> u8 {
        let mut csum: u8 = 0;

        for b in self.as_bytes() {
            csum = csum.wrapping_add(*b);
        }

        255 - csum
    }

    fn new() -> Self
    where
        Self: Sized;
}

/// TODO: move this to the Command enums
pub mod def {
    pub const DATA_CLASS: u8 = 0x3E;
    pub const DATA_BLOCK: u8 = 0x3F;
    pub const BLOCK_DATA_START: u8 = 0x40;
    pub const BLOCK_DATA_CHECKSUM: u8 = 0x60;
    pub const BLOCK_DATA_CONTROL: u8 = 0x61;
}

impl<I, D, E> Bq27xx<I, D>
where
    D: delay::DelayNs,
    I: i2c::I2c<Error = E>,
{
    /// Read the selected block checksum from the gauge
    pub(crate) async fn read_checksum(&mut self) -> Result<u8, ChipError<E>> {
        let mut checksum = [0];

        self.i2c
            .write_read(self.addr, &[def::BLOCK_DATA_CHECKSUM], &mut checksum)
            .await?;

        Ok(checksum[0])
    }

    /// Prepares for the read or write operation
    pub(crate) async fn memblock_prepare_op(
        &mut self,
        class: u8,
        block: u8,
    ) -> Result<(), ChipError<E>> {
        self.i2c
            .write(self.addr, &[def::BLOCK_DATA_CONTROL, 0])
            .await?;

        self.i2c.write(self.addr, &[def::DATA_CLASS, class]).await?;
        self.i2c.write(self.addr, &[def::DATA_BLOCK, block]).await?;

        self.delay.delay_ms(5).await;

        Ok(())
    }

    /// Reads the entire memory block
    pub async fn memblock_read<M: MemoryBlock>(&mut self) -> Result<M, ChipError<E>> {
        let mut block = M::new();

        self.memblock_prepare_op(block.get_class(), 0).await?;

        let checksum = self.read_checksum().await?;

        self.i2c
            .write_read(self.addr, &[def::BLOCK_DATA_START], block.as_bytes_mut())
            .await?;

        if checksum != block.checksum() {
            Err(ChipError::Checksum)
        } else {
            Ok(block)
        }
    }

    /// Writes the entire memory block
    pub async fn memblock_write<M: MemoryBlock>(&mut self, block: &M) -> Result<(), ChipError<E>> {
        let class = block.get_class();

        self.mode_cfgupdate().await?;
        self.memblock_prepare_op(class, 0).await?;

        for (i, ptr) in block.as_bytes().iter().enumerate() {
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
        self.memblock_prepare_op(class, 0).await?;

        let chip_checksum = self.read_checksum().await?;
        self.soft_reset().await?;

        if checksum == chip_checksum {
            Ok(())
        } else {
            Err(ChipError::Checksum)
        }
    }

    /// Helper to modify the memory parameters in place
    pub async fn memory_modify<M, F>(&mut self, f: F) -> Result<(), ChipError<E>>
    where
        M: MemoryBlock,
        F: FnOnce(&mut M),
    {
        let mut memblock: M = self.memblock_read().await?;

        f(&mut memblock);
        self.memblock_write(&memblock).await
    }
}
