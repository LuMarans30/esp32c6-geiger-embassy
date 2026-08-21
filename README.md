# ESP32-C6 Geiger

A working example demonstrating pulse detection from a Geiger board (e.g. RadiationD v1.1 CAJOE) using `esp-hal` and Embassy on the ESP32-C6. 

It uses an async task to detect and log pulses without blocking the main function. To ensure accurate counts, it relies on a software debounce mechanism.

## Wiring

* `GND`: Ground
* `5V`: 5V Power
* `VIN`: Signal output

The RadiationD CAJOE `VIN` output is an open-collector output.
It is idle HIGH and goes LOW on a count.

This firmware defaults to counting falling edges and uses `Pull::None` for the input pin.

> [!NOTE]
> The pin labeled `VIN` on the board silkscreen is the signal output.
> Connect it to a GPIO pin, for example `GPIO4`.

> [!TIP]
> You can disable the beeping by disconnecting the `J1` jumper located above the piezo buzzer.

## Configuration

You can configure the tube divider (CPM per µSv/h) via the USB serial CLI.

First, identify the device and grant access to the serial port:
```bash
# Identify the device
ls -l /dev/ttyUSB* /dev/ttyACM* 2>/dev/null
sudo dmesg | grep -iE 'ttyUSB|ttyACM' | tail   
lsusb

# Grant access (you may need to log out and log back in for this to take effect)
sudo usermod -aG dialout $USER
```

Then, run the following command to access the CLI:
```bash
screen <device> <baud> # e.g. screen /dev/ttyACM0 115200
```

> [!TIP]
> I recommend using [tio](https://github.com/tio/tio) instead of `screen`. 
> Note that unlike `screen`, it is not pre-installed on most distributions.
> Connect using: `tio <device>`

You can now run the following commands:

```text
geiger> help
Commands:
help
divider [value]
tube [name]
pulse [low|high]
reset
```

The stored value is the tube divider in CPM per µSv/h.
```text
geiger> tube
Tube presets (divider = CPM per uSv/h):
j305        153.8
j315        153.8
m4011       153.8
sbm20       175.0
si29bg       91.0
lnd712      108.0
lnd7317      65.0
sts5        116.0
sbt11a      318.0
```

## Pulse Input

The firmware is configured to detect either falling or rising edges on `GPIO4` with internal pull resistors disabled (`Pull::None`). 

You can change the edge detector behavior dynamically via the `pulse` CLI command.
