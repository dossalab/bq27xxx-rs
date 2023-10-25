//! These are low-level definitions for BQ27427 and similar chips

#[cfg(feature = "defmt")]
use defmt::bitflags;

#[cfg(not(feature = "defmt"))]
use bitflags::bitflags;

/// Issuing a Control() command requires a subsequent 2-byte subcommand.
/// Additional bytes specify the particular control function desired

/// A command enum that can hold both simple and extended commands
pub enum Command {
    Simple(u8),
    Control(u8, u16),
}

macro_rules! define_command {
    ($name:ident, $command:expr) => {
        pub const $name: Command = Command::Simple($command);
    };
    ($name:ident, $command:expr, $subcommand:expr) => {
        pub const $name: Command = Command::Control($command, $subcommand);
    };
}

/// This is a list of commands (i.e *registers*) supported by the gauge
pub mod commands {
    #![allow(dead_code)]
    use super::Command;

    // "Simple", i.e 1-byte commands
    define_command!(TEMPERATURE, 0x02);
    define_command!(VOLTAGE, 0x04);
    define_command!(FLAGS, 0x06);
    define_command!(NOMINAL_AVAILABLE_CAPACITY, 0x08);
    define_command!(FULL_AVAILABLE_CAPACITY, 0x0A);
    define_command!(REMAINING_CAPACITY, 0x0C);
    define_command!(FULL_CHARGE_CAPACITY, 0x0E);
    define_command!(AVERAGE_CURRENT, 0x10);
    define_command!(AVERAGE_POWER, 0x18);
    define_command!(STATE_OF_CHARGE, 0x1C);
    define_command!(INTERNAL_TEMPERATURE, 0x1E);
    define_command!(STATE_OF_HEALTH, 0x20);
    define_command!(REMAINING_CAPACITY_UNFILTERED, 0x28);
    define_command!(REMAINING_CAPACITY_FILTERED, 0x2A);
    define_command!(FULL_CHARGE_CAPACITY_UNFILTERED, 0x2C);
    define_command!(FULL_CHARGE_CAPACITY_FILTERED, 0x2E);
    define_command!(STATE_OF_CHARGE_UNFILTERED, 0x30);

    // Control command and it's subcommands
    const CONTROL: u8 = 0x00;

    define_command!(CONTROL_STATUS, CONTROL, 0x0000);
    define_command!(DEVICE_TYPE, CONTROL, 0x0001);
    define_command!(FW_VERSION, CONTROL, 0x0002);
    define_command!(DM_CODE, CONTROL, 0x0004);
    define_command!(PREV_MACWRITE, CONTROL, 0x0007);
    define_command!(CHEM_ID, CONTROL, 0x0008);
    define_command!(BAT_INSERT, CONTROL, 0x000C);
    define_command!(BAT_REMOVE, CONTROL, 0x000D);
    define_command!(SET_CFGUPDATE, CONTROL, 0x0013);
    define_command!(SMOOTH_SYNC, CONTROL, 0x0019);
    define_command!(SHUTDOWN_ENABLE, CONTROL, 0x001B);
    define_command!(SHUTDOWN, CONTROL, 0x001C);
    define_command!(SEALED, CONTROL, 0x0020);
    define_command!(PULSE_SOC_INT, CONTROL, 0x0023);
    define_command!(CHEM_A, CONTROL, 0x0030);
    define_command!(CHEM_B, CONTROL, 0x0031);
    define_command!(CHEM_C, CONTROL, 0x0032);
    define_command!(RESET, CONTROL, 0x0041);
    define_command!(SOFT_RESET, CONTROL, 0x0042);
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

impl From<u16> for StatusFlags {
    fn from(value: u16) -> Self {
        StatusFlags::from_bits_truncate(value)
    }
}
