# ESP32-C6 Geiger

A working example demonstrating pulse detection from a Geiger board (e.g., RadiationD v1.1 CAJOE) using `esp-hal` and Embassy on the ESP32-C6. 

It uses an async task to detect and log pulses without blocking the main function. To ensure accurate counts, it relies on a software debounce mechanism.

## Wiring

* `GND`: Ground
* `5V`: 5V Power
* `VIN`: Signal output
  * **Note**: This pin is mislabeled on the silkscreen. Connect this to a GPIO pin (e.g. `GPIO4`)

> [!TIP]
> You can disable the beeping by disconnecting the `J1` jumper, which is located above the piezo buzzer.

## Configuration

The tube conversion ratio can be adjusted in `main.rs` via the `CPM_RATIO` constant:
* `318.0` for SBT-11A
* `153.8` for M4011 / J305 (the default CAJOE tube)