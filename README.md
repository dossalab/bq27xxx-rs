## BQ27xxx (bq27426, bq27427) driver written in Rust

BQ27xxx is a Texas Instruments (TI) series of fuel gauges - i.e special kind of chips used to monitor the health of lithium batteries.

### Rust + async ❤️ embedded

This exact driver was developed on BQ27427 - a rather new part with integrated sensing resistor. I suspect that other chips can (or already are) supported - feel free to send me the patch if you notice any differences.
The I2C interface here is slow and a bit quirky, so some commands may take seconds of processing time. In 'traditional' C-style driver this either requires blocking (i.e waiting until each operation is completed), or writing a state machine with state transition and error handling logic. Writing such driver is async Rust makes everything so much easier!

### Features
- Uses embedded-hal async traits for compatability with a variety of hardware;
- Reading basic parameters such as voltage, temperature, state-of-charge and so on;
- Setting battery chemistry, executing basic commands.

### TODO
- The chip has direct memory access which allows changing parameters such as design capacity. This API is not fully completed yet :(
