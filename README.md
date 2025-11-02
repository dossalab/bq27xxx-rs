## BQ27xxx (bq27426, bq27427) driver written in Rust

BQ27xxx is a Texas Instruments (TI) series of fuel gauges - i.e special kind of chips used to monitor the health of lithium batteries.

### Rust + async ❤️ embedded

This exact driver was developed on BQ27427 - a rather new part with integrated sensing resistor. I suspect that other chips can (or already are) supported - feel free to send me the patch if you notice any differences.
The I2C interface here is slow and a bit quirky, so some commands may take seconds of processing time. In 'traditional' C-style driver this either requires blocking (i.e waiting until each operation is completed), or writing a state machine with state transition and error handling logic. Writing such driver is async Rust makes everything so much easier!

### Features
- Uses embedded-hal async traits for compatability with a variety of hardware;
- Reading basic parameters such as voltage, temperature, state-of-charge and so on;
- Setting battery chemistry, executing basic commands.

## Demo

Basic usage:

```
const GAUGE_I2C_ADDR: u8 = 0x55;

let mut gauge = Bq27xx::new(i2c, embassy_time::Delay, GAUGE_I2C_ADDR),
gauge.probe().await?;

info!("SoC: {} %", self.gauge.state_of_charge().await?);
info!("V: {} mV", self.gauge.voltage().await?);
info!("I: {} mA", self.gauge.average_current().await?);
info!("Flags: {}", self.gauge.get_flags().await?);
```

### Memory block interface

Memory layout is specific to each concrete part number and even close parts might have differences. Enable feature appropriate to your gauge (i.e bq27427) - that will import the required defines.

Dumping the blocks (note the type annotation):
```
info!(
    "state: {}",
    self.gauge.memblock_read::<bq27427::StateClassBlock>().await?
);

info!("ratable: {}", self.gauge.memblock_read::<bq27427::RaTable>().await?);
```

Setting the memory params:
```
self.gauge
    .memory_modify(|b: &mut bq27427::StateClass| {
        b.set_capacity(200); // mAh
        b.set_energy(740); // Capacity * 3.7
        b.set_terminate_voltage(3200); // mV

        // Capacity / (0.1 × taper current)
        b.set_taper_rate(75);

        // QMax value from the learning cycle
        b.set_qmax(17449);
    })
    .await?;

self.gauge
    .memory_modify(|b: &mut bq27427::RaTable| {
        // Ra table from the learning cycle
        b.set_points([50, 30, 34, 46, 38, 32, 37, 31, 32, 35, 39, 39, 61, 115, 200]);
    })
    .await
```

## Usage note

Don't expect the gauge to be plug-n-play. In order to get the sensible reading, the gauge needs to know at least:

- Taper rate (current) - depends on your charger,
- Battery chemistry, capacity and energy - depends on your battery,
- Terminate voltage - depends on your battery and system requirements

In addition to that, you have to complete the learning cycle. As a result, you'll get:

- QMAX parameter,
- RA table.

Which you should include into your production image.

### Taper rate

Taper rate is an important parameter that might require tweaking. After programming some sensible starting parameters (refer to the TRM on how to calculate them!), you'll have to do a little experiment:

- Try charging the battery to the full and see if the gauge detects 100% charge.

This step is absolutely crucial. If the taper rate is set incorrectly, the gauge will never report 100% and will always stuck at 99%.

If it gets stuck and never reports full charge, try decreasing the taper rate param (increasing the taper rate current). My charger (nPM1100) with 100 mA fast charge current tapers at around 25 mA (even though that is well above specified limit in the datasheet).

If you pass this step you can move on to a learning cycle.

### Learning cycle

(I'm no expert here so take everything with a grain of salt)

Roughly speaking you need to allow the chip to take QMAX measurement and update its RA table.

To update QMAX, chip needs to take 2 OCV measurements with enough difference in charge (i.e say one measurement in discharged state, and one in charged).

OCV is an open-circuit measurement which is taken during relaxation period (when battery is neither charging nor discharging).

Possible caveats here:

- During relaxation time your system should absolutely not pull current from the battery. It seems ok to leave the MCU running in sleep mode, but if you have debugger attached or have any peripherals running and your MCU is consuming as low as 5-7 mA gauge will never take an OCV measurement. I'd recomment to power your jig from another source during calibration to not have this issue.
