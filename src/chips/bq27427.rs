use embedded_hal_async::{delay, i2c};

#[cfg(feature = "defmt")]
use defmt::{debug, info};

use crate::{
    memory::{self, MemoryBlock},
    Bq27xx, ChipError,
};

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

        self.memblock_prepare_op(memory_subclass::CC_CAL, 0).await?;

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

pub struct StateClassBlock {
    inner: [u8; 32],
}

impl MemoryBlock for StateClassBlock {
    fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    fn new() -> StateClassBlock {
        StateClassBlock { inner: [0; 32] }
    }

    fn get_class(&self) -> u8 {
        memory_subclass::STATE
    }
}

impl StateClassBlock {
    const QMAX: usize = 0;
    const UPDATE_STATUS: usize = 2;
    const CAPACITY: usize = 6;
    const ENERGY: usize = 8;
    const TERMINATE_VOLTAGE: usize = 10;
    const TAPER_RATE: usize = 21;

    pub fn set_qmax(&mut self, qmax: u16) {
        self.write_u16(Self::QMAX, qmax)
    }

    pub fn get_qmax(&self) -> u16 {
        self.read_u16(Self::QMAX)
    }

    pub fn set_capacity(&mut self, capacity: u16) {
        self.write_u16(Self::CAPACITY, capacity)
    }

    pub fn get_capacity(&self) -> u16 {
        self.read_u16(Self::CAPACITY)
    }

    pub fn set_energy(&mut self, energy: u16) {
        self.write_u16(Self::ENERGY, energy)
    }

    pub fn get_energy(&self) -> u16 {
        self.read_u16(Self::ENERGY)
    }

    pub fn set_taper_rate(&mut self, rate: u16) {
        self.write_u16(Self::TAPER_RATE, rate)
    }

    pub fn get_taper_rate(&self) -> u16 {
        self.read_u16(Self::TAPER_RATE)
    }

    pub fn set_terminate_voltage(&mut self, v: u16) {
        self.write_u16(Self::TERMINATE_VOLTAGE, v)
    }

    pub fn get_terminate_voltage(&self) -> u16 {
        self.read_u16(Self::TERMINATE_VOLTAGE)
    }

    pub fn set_update_status(&mut self, status: u8) {
        self.write_u8(Self::UPDATE_STATUS, status);
    }

    pub fn get_update_status(&self) -> u8 {
        self.read_u8(Self::UPDATE_STATUS)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for StateClassBlock {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "capacity {} mAh, energy {} mWh, taper rate {} .1Hr, terminate voltage {} mV, QMAX {}, update status {}",
            self.get_capacity(),
            self.get_energy(),
            self.get_taper_rate(),
            self.get_terminate_voltage(),
            self.get_qmax(),
            self.get_update_status()
        );
    }
}

pub struct CurrentThresholds {
    inner: [u8; 32],
}

impl MemoryBlock for CurrentThresholds {
    fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    fn new() -> CurrentThresholds {
        CurrentThresholds { inner: [0; 32] }
    }

    fn get_class(&self) -> u8 {
        memory_subclass::CURRENT_THRESHOLDS
    }
}

impl CurrentThresholds {
    pub fn set_discharge_current_threshold(&mut self, v: u16) {
        self.write_u16(0, v)
    }

    pub fn get_discharge_current_threshold(&self) -> u16 {
        self.read_u16(0)
    }

    pub fn set_quit_current_threshold(&mut self, v: u16) {
        self.write_u16(4, v)
    }

    pub fn get_quit_current_threshold(&self) -> u16 {
        self.read_u16(4)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for CurrentThresholds {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "discharge_current_thresh: {}, quit_current_thresh: {}",
            self.get_discharge_current_threshold(),
            self.get_quit_current_threshold()
        );
    }
}

pub struct RaTable {
    inner: [u8; 32],
}

impl MemoryBlock for RaTable {
    fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    fn new() -> Self {
        Self { inner: [0; 32] }
    }

    fn get_class(&self) -> u8 {
        memory_subclass::RA0_RAM
    }
}

impl RaTable {
    pub fn set_points(&mut self, points: [u16; 15]) {
        for (i, p) in points.iter().enumerate() {
            self.write_u16(i * 2, *p)
        }
    }

    pub fn get_points(&self) -> [u16; 15] {
        core::array::from_fn(|i| self.read_u16(i * 2))
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for RaTable {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "points: {}", self.get_points(),);
    }
}
