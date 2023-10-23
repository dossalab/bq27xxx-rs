## BQ27xxx (bq27426, bq27427) driver written in Rust

BQ27xxx is a Texas Instruments (TI) series of fuel gauges - i.e special kind of chips used to monitor the health of lithium batteries.

### Rust + async ❤️ embedded

This exact driver was developed on BQ27427 - a rather new part with integrated sensing resistor. I suspect that other chips can (or already are) supported - feel free to send me the patch if you notice any differences.
The interface they use is quirky and includes many consecutive reads and writes that take seconds of processing time. Writing state machines in plain C is fun and all, but we all gathered here to make cool devices, right? Async / await to the rescue!

### Features
- Uses embedded-hal async (nightly) traits for compatability with various hardware;
