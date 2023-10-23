#![no_std]

//! A small driver for Texas Instruments battery gauges (written for BQ27427)

pub mod known_chips;

pub(crate) mod fmt;
pub(crate) mod registers;

use byteorder::{BigEndian, ByteOrder};
use embedded_hal_async::{delay, i2c};
use fmt::*;
use known_chips::ChipType;
use registers::*;

/// Battery chemistry type. B4200 should be suited for the most hobby-grade cells
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChemId {
    A4350,
    B4200,
    C4400,
    Unknown,
}

impl From<u16> for ChemId {
    fn from(code: u16) -> Self {
        match code {
            0x3230 => Self::A4350,
            0x1202 => Self::B4200,
            0x3142 => Self::C4400,
            _ => Self::Unknown,
        }
    }
}

const MEMBLOCK_SIZE: usize = 32;

/// Chip error type
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChipError<E> {
    I2CError(E),
    PollTimeout,
    Checksum,
}

impl<E> From<E> for ChipError<E> {
    fn from(e: E) -> Self {
        Self::I2CError(e)
    }
}

/// Chip handle
pub struct Bq27xx<I, D> {
    i2c: I,
    delay: D,
    addr: u8,
}

impl<I, D, E> Bq27xx<I, D>
where
    D: delay::DelayUs,
    I: i2c::I2c<Error = E>,
{
    /*
     * The protocol is wacky - see the datasheet for the details. Long story short
     * the chip does not like long transactions so writing the command and getting the
     * response are 2 different operations.
     */
    async fn read_control(&mut self, subcommand: u16) -> Result<u16, ChipError<E>> {
        let mut response = [0, 0];
        let request = [commands::CONTROL, subcommand as u8, (subcommand >> 8) as u8];

        self.i2c.write(self.addr, &request).await?;
        self.i2c
            .write_read(self.addr, &[commands::CONTROL], &mut response)
            .await?;

        Ok(u16::from_le_bytes(response))
    }

    async fn write_control(&mut self, subcommand: u16) -> Result<(), ChipError<E>> {
        let request = [commands::CONTROL, subcommand as u8, (subcommand >> 8) as u8];
        self.i2c.write(self.addr, &request).await?;
        Ok(())
    }

    async fn read_command(&mut self, command: u8) -> Result<u16, ChipError<E>> {
        let mut response = [0, 0];

        self.i2c
            .write_read(self.addr, &[command], &mut response)
            .await?;

        Ok(u16::from_le_bytes(response))
    }

    async fn write_command(&mut self, command: u8, data: u8) -> Result<(), ChipError<E>> {
        let request = [command, data];
        self.i2c.write(self.addr, &request).await?;
        Ok(())
    }

    /// Reads the contents of the Flags register
    pub async fn get_flags(&mut self) -> Result<StatusFlags, ChipError<E>> {
        let raw = self.read_command(commands::FLAGS).await?;
        Ok(StatusFlags::from_bits_truncate(raw))
    }

    /// Waits for any of given flags (provided as a mask)
    async fn wait_flags(&mut self, mask: StatusFlags) -> Result<(), ChipError<E>> {
        const FLAG_POLL_RETRIES: u32 = 10;
        const FLAG_POLL_DELAY_MS: u32 = 500;

        for _ in 0..FLAG_POLL_RETRIES {
            self.delay.delay_ms(FLAG_POLL_DELAY_MS).await;

            let flags = self.get_flags().await?;

            if flags.contains(mask) {
                return Ok(());
            }
        }

        Err(ChipError::PollTimeout)
    }

    /// Moves the chip to CFGUP mode
    async fn mode_cfgupdate(&mut self) -> Result<(), ChipError<E>> {
        info!("entering cfgupdate mode...");

        self.write_control(control_subcommands::SET_CFGUPDATE)
            .await?;
        self.wait_flags(StatusFlags::CFGUPMODE).await?;

        info!("cfgupdate mode entered!");

        Ok(())
    }

    /// Besides regular commands and control commands there is also direct
    /// memory access using datablocks - some parameters are only
    /// available through that interface. Bravo TI!

    /// Simple checksum used by the gauge
    fn calculate_block_checksum(&self, block: &[u8]) -> u8 {
        let mut csum: u8 = 0;

        for b in block.iter() {
            csum = csum.wrapping_add(*b);
        }

        255 - csum
    }

    /// Read the selected block checksum from the gauge
    async fn read_checksum(&mut self) -> Result<u8, ChipError<E>> {
        let mut checksum = [0];

        self.i2c
            .write_read(self.addr, &[commands::BLOCK_DATA_CHECKSUM], &mut checksum)
            .await?;

        Ok(checksum[0])
    }

    /// Prepares for the read or write operation. Leaves the device in CFGUPG mode
    async fn memblock_prepare_op(&mut self, class: u8, block: u8) -> Result<(), ChipError<E>> {
        self.write_command(commands::BLOCK_DATA_CONTROL, 0).await?;
        self.write_command(commands::DATA_CLASS, class).await?;
        self.write_command(commands::DATA_BLOCK, block).await?;

        self.delay.delay_ms(5).await;

        Ok(())
    }

    async fn memblock_read(
        &mut self,
        class: u8,
        block: u8,
        data: &mut [u8; MEMBLOCK_SIZE],
    ) -> Result<(), ChipError<E>> {
        self.memblock_prepare_op(class, block).await?;

        let checksum = self.read_checksum().await?;
        self.i2c
            .write_read(self.addr, &[commands::BLOCK_DATA], data)
            .await?;

        if checksum != self.calculate_block_checksum(data) {
            Err(ChipError::Checksum)
        } else {
            Ok(())
        }
    }

    async fn memblock_write(
        &mut self,
        class: u8,
        block: u8,
        data: &mut [u8; MEMBLOCK_SIZE],
    ) -> Result<(), ChipError<E>> {
        let checksum = self.calculate_block_checksum(data);

        self.mode_cfgupdate().await?;
        self.memblock_prepare_op(class, block).await?;

        self.i2c.write(self.addr, &[commands::BLOCK_DATA]).await?;

        info!("writing datablock...");

        for (index, byte) in data.iter().enumerate() {
            self.i2c
                .write(self.addr, &[commands::BLOCK_DATA + index as u8, *byte])
                .await?;
        }

        // Propose our computed checksum to the chip
        self.i2c
            .write(self.addr, &[commands::BLOCK_DATA_CHECKSUM, checksum])
            .await?;

        // And let it think for a bit....
        self.delay.delay_ms(5).await;

        // Here we go, the chip had enough time to think, we can double check if
        // everything is correct and exit cfgupdate

        self.memblock_prepare_op(class, block).await?;

        let chip_checksum = self.read_checksum().await?;
        self.soft_reset().await?;

        if checksum == chip_checksum {
            Ok(())
        } else {
            Err(ChipError::Checksum)
        }
    }

    pub async fn write_chem_id(&mut self, id: ChemId) -> Result<(), ChipError<E>> {
        self.mode_cfgupdate().await?;

        let subcommand = match id {
            ChemId::A4350 => control_subcommands::CHEM_A,
            ChemId::B4200 => control_subcommands::CHEM_B,
            ChemId::C4400 => control_subcommands::CHEM_C,
            ChemId::Unknown => panic!("cannot set unknown chem id!"),
        };

        self.write_control(subcommand).await?;
        self.soft_reset().await
    }

    pub async fn read_chem_id(&mut self) -> Result<ChemId, ChipError<E>> {
        let response = self.read_control(control_subcommands::CHEM_ID).await?;
        Ok(ChemId::from(response))
    }

    pub async fn get_programmed_capacity(&mut self) -> Result<u16, ChipError<E>> {
        let mut block: [u8; MEMBLOCK_SIZE] = Default::default();

        self.memblock_read(memory_subclass::STATE, 0, &mut block)
            .await?;

        Ok(BigEndian::read_u16(&block[6..8]))
    }

    pub async fn set_programmed_capacity(&mut self, capacity: u16) -> Result<(), ChipError<E>> {
        let mut block: [u8; MEMBLOCK_SIZE] = Default::default();

        self.memblock_read(memory_subclass::STATE, 0, &mut block)
            .await?;

        BigEndian::write_u16(&mut block[6..8], capacity);

        self.memblock_write(memory_subclass::STATE, 0, &mut block)
            .await
    }

    /// Hard resets the chip. Re-initializes the memory with default values,
    /// so further configuration is required.
    pub async fn reset(&mut self) -> Result<(), ChipError<E>> {
        info!("performing hard reset...");
        self.write_control(control_subcommands::RESET).await
    }

    /// Partial reset. Memory is not cleared. Useful for field operation.
    pub async fn soft_reset(&mut self) -> Result<(), ChipError<E>> {
        info!("performing soft reset...");
        self.write_control(control_subcommands::SOFT_RESET).await
    }

    pub async fn state_of_charge(&mut self) -> Result<u16, ChipError<E>> {
        self.read_command(commands::STATE_OF_CHARGE).await
    }

    /// Reads the battery voltage in millivolts
    pub async fn voltage(&mut self) -> Result<u16, ChipError<E>> {
        self.read_command(commands::VOLTAGE).await
    }

    pub async fn average_current(&mut self) -> Result<i16, ChipError<E>> {
        let raw = self.read_command(commands::AVERAGE_CURRENT).await?;
        Ok(raw as i16)
    }

    /// Reads the temperature sensor, either internal or external, depending on the configuration
    pub async fn temperature(&mut self) -> Result<u16, ChipError<E>> {
        self.read_command(commands::TEMPERATURE).await
    }

    /// Gets the firmware version
    pub async fn fw_version(&mut self) -> Result<u16, ChipError<E>> {
        self.read_control(control_subcommands::FW_VERSION).await
    }

    async fn check_bq27427_errata(&mut self) -> Result<(), ChipError<E>> {
        let mut block: [u8; MEMBLOCK_SIZE] = Default::default();

        info!("bq27427: checking errata...");

        self.memblock_read(memory_subclass::CC_CAL, 0, &mut block)
            .await?;

        if block[5] & 0x80 != 0 {
            info!("bq27427: the CC calibration is negative");
            block[5] &= !0x80;

            info!("applying corrected cc calibration - {:x}", &block[4..8]);
        }

        self.memblock_write(memory_subclass::CC_CAL, 0, &mut block)
            .await?;

        Ok(())
    }

    /// Tries to communicate with the chip and reads the device type
    pub async fn probe(&mut self) -> Result<ChipType, ChipError<E>> {
        let response = self.read_control(control_subcommands::DEVICE_TYPE).await?;

        let device_type = ChipType::from(response);
        match device_type {
            ChipType::BQ27427 => self.check_bq27427_errata().await?,
            _ => {}
        }

        Ok(device_type)
    }

    /// Creates the driver instance
    pub fn new(i2c: I, delay: D, addr: u8) -> Self {
        Self { i2c, addr, delay }
    }
}
