#![no_std]

//! A small driver for Texas Instruments battery gauges (written for BQ27427)

pub mod known_chips;
pub mod memory;

pub(crate) mod defs;

use defs::*;
use embedded_hal_async::{delay, i2c};
use known_chips::ChipType;

/// The error type
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChipError<E> {
    I2CError(E),
    PollTimeout,
    Checksum,
    Value,
    Usage,
}

impl<E> From<E> for ChipError<E> {
    fn from(e: E) -> Self {
        Self::I2CError(e)
    }
}

/// Battery chemistry type. B4200 should be suited for the most hobby-grade cells
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChemId {
    A4350,
    B4200,
    C4400,
    Unknown,
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
    // Reads the data from either simple or control command.
    async fn read<R>(&mut self, envelope: Command) -> Result<R, ChipError<E>>
    where
        R: TryFrom<u16>,
    {
        let mut response = [0, 0];

        match envelope {
            Command::Simple(command) => {
                self.i2c
                    .write_read(self.addr, &[command], &mut response)
                    .await?
            }
            Command::Control(command, subcommand) => {
                let [b1, b2] = subcommand.to_le_bytes();

                self.i2c.write(self.addr, &[command, b1, b2]).await?;
                self.i2c
                    .write_read(self.addr, &[command], &mut response)
                    .await?;
            }
        }

        u16::from_le_bytes(response)
            .try_into()
            .map_err(|_| ChipError::Value)
    }

    // Executes the command. Typically not used for simple commands
    async fn execute(&mut self, envelope: Command) -> Result<(), ChipError<E>> {
        match envelope {
            Command::Control(command, subcommand) => {
                let [b1, b2] = subcommand.to_le_bytes();
                self.i2c
                    .write(self.addr, &[command, b1, b2])
                    .await
                    .map_err(|e| ChipError::I2CError(e))
            }

            Command::Simple(_) => Err(ChipError::Value),
        }
    }

    /// Reads the contents of the Flags register
    pub async fn get_flags(&mut self) -> Result<StatusFlags, ChipError<E>> {
        self.read(commands::FLAGS).await
    }

    /// Waits for any of the given flags (provided as bitmask)
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

    /// Moves the chip to configuration update mode. Required for writing settings
    async fn mode_cfgupdate(&mut self) -> Result<(), ChipError<E>> {
        self.execute(commands::SET_CFGUPDATE).await?;
        self.wait_flags(StatusFlags::CFGUPMODE).await
    }

    pub async fn write_chem_id(&mut self, id: ChemId) -> Result<(), ChipError<E>> {
        self.mode_cfgupdate().await?;

        let result = self
            .execute(match id {
                ChemId::A4350 => commands::CHEM_A,
                ChemId::B4200 => commands::CHEM_B,
                ChemId::C4400 => commands::CHEM_C,
                _ => return Err(ChipError::Value),
            })
            .await;

        self.soft_reset().await?;
        result
    }

    /// Reads the currently selected chem id
    pub async fn read_chem_id(&mut self) -> Result<ChemId, ChipError<E>> {
        match self.read(commands::CHEM_ID).await? {
            0x3230 => Ok(ChemId::A4350),
            0x1202 => Ok(ChemId::B4200),
            0x3142 => Ok(ChemId::C4400),
            _ => Err(ChipError::Value),
        }
    }

    /// Hard resets the chip. Re-initializes the memory with default values,
    /// so further configuration is required.
    pub async fn reset(&mut self) -> Result<(), ChipError<E>> {
        self.execute(commands::RESET).await
    }

    /// Partial reset. Memory is not cleared. Useful for field operation.
    pub async fn soft_reset(&mut self) -> Result<(), ChipError<E>> {
        self.execute(commands::SOFT_RESET).await
    }

    /// Reads the state of charge
    pub async fn state_of_charge(&mut self) -> Result<u16, ChipError<E>> {
        self.read(commands::STATE_OF_CHARGE).await
    }

    /// Reads the battery voltage in millivolts
    pub async fn voltage(&mut self) -> Result<u16, ChipError<E>> {
        self.read(commands::VOLTAGE).await
    }

    /// Reads the average current
    pub async fn average_current(&mut self) -> Result<i16, ChipError<E>> {
        self.read(commands::AVERAGE_CURRENT).await
    }

    /// Reads the temperature sensor, either internal or external, depending on the configuration
    pub async fn temperature(&mut self) -> Result<u16, ChipError<E>> {
        self.read(commands::TEMPERATURE).await
    }

    /// Reads the firmware version
    pub async fn fw_version(&mut self) -> Result<u16, ChipError<E>> {
        self.read(commands::FW_VERSION).await
    }

    /// Reads the device type
    pub async fn device_type(&mut self) -> Result<ChipType, ChipError<E>> {
        self.read(commands::DEVICE_TYPE).await
    }

    /// Creates the driver instance
    pub fn new(i2c: I, delay: D, addr: u8) -> Self {
        Self { i2c, addr, delay }
    }
}
