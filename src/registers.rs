//! These are low-level definitions for BQ27427 and similar chips

use crate::fmt::bitflags;

/// This is a list of commands (i.e *registers*) supported by the gauge
pub mod commands {
    #![allow(dead_code)]
    pub const CONTROL: u8 = 0x00;
    pub const TEMPERATURE: u8 = 0x02;
    pub const VOLTAGE: u8 = 0x04;
    pub const FLAGS: u8 = 0x06;
    pub const NOMINAL_AVAILABLE_CAPACITY: u8 = 0x08;
    pub const FULL_AVAILABLE_CAPACITY: u8 = 0x0A;
    pub const REMAINING_CAPACITY: u8 = 0x0C;
    pub const FULL_CHARGE_CAPACITY: u8 = 0x0E;
    pub const AVERAGE_CURRENT: u8 = 0x10;
    pub const AVERAGE_POWER: u8 = 0x18;
    pub const STATE_OF_CHARGE: u8 = 0x1C;
    pub const INTERNAL_TEMPERATURE: u8 = 0x1E;
    pub const STATE_OF_HEALTH: u8 = 0x20;
    pub const REMAINING_CAPACITY_UNFILTERED: u8 = 0x28;
    pub const REMAINING_CAPACITY_FILTERED: u8 = 0x2A;
    pub const FULL_CHARGE_CAPACITY_UNFILTERED: u8 = 0x2C;
    pub const FULL_CHARGE_CAPACITY_FILTERED: u8 = 0x2E;
    pub const STATE_OF_CHARGE_UNFILTERED: u8 = 0x30;

    // Extended, i.e direct memory access
    pub const DATA_CLASS: u8 = 0x3E;
    pub const DATA_BLOCK: u8 = 0x3F;
    pub const BLOCK_DATA: u8 = 0x40;
    pub const BLOCK_DATA_CHECKSUM: u8 = 0x60;
    pub const BLOCK_DATA_CONTROL: u8 = 0x61;
}

/// All memory locations in turn are divided into subclasses
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

/// Issuing a Control() command requires a subsequent 2-byte subcommand.
/// Additional bytes specify the particular control function desired
pub mod control_subcommands {
    #![allow(dead_code)]
    pub const CONTROL_STATUS: u16 = 0x0000;
    pub const DEVICE_TYPE: u16 = 0x0001;
    pub const FW_VERSION: u16 = 0x0002;
    pub const DM_CODE: u16 = 0x0004;
    pub const PREV_MACWRITE: u16 = 0x0007;
    pub const CHEM_ID: u16 = 0x0008;
    pub const BAT_INSERT: u16 = 0x000C;
    pub const BAT_REMOVE: u16 = 0x000D;
    pub const SET_CFGUPDATE: u16 = 0x0013;
    pub const SMOOTH_SYNC: u16 = 0x0019;
    pub const SHUTDOWN_ENABLE: u16 = 0x001B;
    pub const SHUTDOWN: u16 = 0x001C;
    pub const SEALED: u16 = 0x0020;
    pub const PULSE_SOC_INT: u16 = 0x0023;
    pub const CHEM_A: u16 = 0x0030;
    pub const CHEM_B: u16 = 0x0031;
    pub const CHEM_C: u16 = 0x0032;
    pub const RESET: u16 = 0x0041;
    pub const SOFT_RESET: u16 = 0x0042;
}

bitflags! {
    /// Contents of the flags register, returned by the "Flags" command
    pub struct StatusFlags: u16 {
        const OT = 1 << 15;
        const UT = 1 << 14;
        const FC = 1 << 9;
        const CHG = 1 << 8;
        const OCVTAKEN = 1 << 7;
        const DOD_CORRECT = 1 << 6;
        const ITPOR = 1 << 5;
        const CFGUPMODE = 1 << 4;
        const BAT_DET = 1 << 3;
        const SOC1 = 1 << 2;
        const SOCF = 1 << 1;
        const DSG = 1 << 0;
    }
}
