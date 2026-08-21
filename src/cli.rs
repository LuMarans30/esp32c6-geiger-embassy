use crate::config::{self, ConfigStore, PulsePolarity};
use crate::tube;
use alloc::format;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};

pub type Serial = esp_hal::usb_serial_jtag::UsbSerialJtag<'static, esp_hal::Async>;

const BANNER: &str = "\r\nGeiger CLI\r\nCommands: help, divider, tube, pulse, reset\r\n";
const PROMPT: &str = "geiger> ";

pub fn spawn(spawner: Spawner, serial: Serial, config: ConfigStore) {
    spawner.spawn(cli_task(serial, config).unwrap());
}

#[embassy_executor::task]
async fn cli_task(mut serial: Serial, mut config: ConfigStore) {
    let mut line = [0u8; 96];
    let mut skip_lf = false;

    write_str(&mut serial, BANNER).await;

    loop {
        write_str(&mut serial, PROMPT).await;

        let len = read_line(&mut serial, &mut line, &mut skip_lf).await;
        write_str(&mut serial, "\r\n").await;

        if len > 0 {
            match core::str::from_utf8(&line[..len]) {
                Ok(s) => handle_line(s.trim(), &mut serial, &mut config).await,
                Err(_) => write_str(&mut serial, "invalid input\r\n").await,
            }
        }
    }
}

async fn read_line(serial: &mut Serial, line: &mut [u8], skip_lf: &mut bool) -> usize {
    let mut len = 0usize;
    loop {
        let mut byte = [0u8; 1];
        let n = match serial.read(&mut byte).await {
            Ok(n) => n,
            Err(_) => {
                Timer::after(Duration::from_millis(10)).await;
                continue;
            }
        };
        if n == 0 {
            Timer::after(Duration::from_millis(10)).await;
            continue;
        }
        let b = byte[0];
        match b {
            b'\r' => {
                *skip_lf = true;
                return len;
            }
            b'\n' => {
                if *skip_lf {
                    *skip_lf = false;
                    continue;
                }
                return len;
            }
            _ => *skip_lf = false,
        }
        match b {
            0x08 | 0x7F => {
                if len > 0 {
                    len -= 1;
                    let _ = serial.write_all(b"\x08 \x08").await;
                }
            }
            c if (0x20..0x7F).contains(&c) && len < line.len() => {
                line[len] = c;
                len += 1;
                let _ = serial.write_all(&[c]).await;
            }
            _ => {}
        }
    }
}

async fn handle_line(line: &str, serial: &mut Serial, config: &mut ConfigStore) {
    match parse(line) {
        Ok(Some(command)) => execute(command, serial, config).await,
        Ok(None) => {}
        Err(message) => {
            let msg = format!("error: {}\r\n", message);
            write_str(serial, &msg).await;
        }
    }
}

enum Command {
    Help,
    GetDivider,
    SetDivider(f32),
    ListTubes,
    SetTube(tube::TubePreset),
    GetPulse,
    SetPulse(PulsePolarity),
    Reset,
}

fn parse(line: &str) -> Result<Option<Command>, &'static str> {
    let mut parts = line.split_whitespace();

    Ok(match parts.next() {
        None => None,
        Some("help") => Some(Command::Help),
        Some("divider" | "ratio") => match parts.next() {
            None => Some(Command::GetDivider),
            Some(value) => {
                let divider: f32 = value
                    .parse()
                    .map_err(|_| "usage: divider <positive float>")?;
                if !config::is_valid_divider(divider) {
                    return Err("divider must be finite, > 0, and <= 1_000_000");
                }
                Some(Command::SetDivider(divider))
            }
        },
        Some("tube") => match parts.next() {
            None => Some(Command::ListTubes),
            Some(name) => {
                let preset = tube::find(name).ok_or("unknown tube; use `tube` to list presets")?;
                Some(Command::SetTube(preset))
            }
        },
        Some("pulse") => match parts.next() {
            None => Some(Command::GetPulse),
            Some("low" | "falling") => Some(Command::SetPulse(PulsePolarity::ActiveLow)),
            Some("high" | "rising") => Some(Command::SetPulse(PulsePolarity::ActiveHigh)),
            Some(_) => return Err("usage: pulse [low|high]"),
        },
        Some("reset") => Some(Command::Reset),
        Some(_) => return Err("unknown command"),
    })
}

async fn execute(command: Command, serial: &mut Serial, config: &mut ConfigStore) {
    match command {
        Command::Help => {
            write_str(
                serial,
                "Commands:\r\n\
                 help\r\n\
                 divider [value]\r\n\
                 tube [name]\r\n\
                 pulse [low|high]\r\n\
                 reset\r\n",
            )
            .await;
        }
        Command::GetDivider => {
            let msg = format!("divider: {:.2} CPM per uSv/h\r\n", config.divider());
            write_str(serial, &msg).await;
        }
        Command::SetDivider(divider) => match config.save_divider(divider) {
            Ok(()) => write_str(serial, &format!("saved divider: {:.2}\r\n", divider)).await,
            Err(e) => write_str(serial, &format!("error: {}\r\n", e)).await,
        },
        Command::ListTubes => {
            write_str(serial, "Tube presets (divider = CPM per uSv/h):\r\n").await;
            for preset in tube::PRESETS.iter() {
                let msg = format!("{:<10} {:>6.1}\r\n", preset.name, preset.divider);
                write_str(serial, &msg).await;
            }
        }
        Command::SetTube(preset) => match config.save_divider(preset.divider) {
            Ok(()) => {
                write_str(
                    serial,
                    &format!(
                        "saved tube {} divider: {:.1}\r\n",
                        preset.name, preset.divider
                    ),
                )
                .await
            }
            Err(e) => write_str(serial, &format!("error: {}\r\n", e)).await,
        },
        Command::GetPulse => {
            let msg = format!(
                "pulse polarity: {}\r\n",
                match config.polarity() {
                    PulsePolarity::ActiveLow => "active low (falling edge)",
                    PulsePolarity::ActiveHigh => "active high (rising edge)",
                }
            );
            write_str(serial, &msg).await;
        }
        Command::SetPulse(polarity) => match config.save_polarity(polarity) {
            Ok(()) => {
                let msg = format!(
                    "saved pulse polarity: {}\r\n",
                    match polarity {
                        PulsePolarity::ActiveLow => "active low",
                        PulsePolarity::ActiveHigh => "active high",
                    }
                );
                write_str(serial, &msg).await;
            }
            Err(e) => write_str(serial, &format!("error: {}\r\n", e)).await,
        },
        Command::Reset => match config.reset() {
            Ok(()) => {
                write_str(
                    serial,
                    &format!(
                        "reset to defaults (divider: {:.1}, pulse: low)\r\n",
                        config::DEFAULT_DIVIDER
                    ),
                )
                .await
            }
            Err(e) => write_str(serial, &format!("error: {}\r\n", e)).await,
        },
    }
}

async fn write_str(serial: &mut Serial, s: &str) {
    let _ = serial.write_all(s.as_bytes()).await;
}
