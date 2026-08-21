use crate::tube;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;

pub const DEFAULT_DIVIDER: f32 = tube::DEFAULT_DIVIDER;
pub const DEFAULT_POLARITY: PulsePolarity = PulsePolarity::ActiveLow;

const MAGIC: u32 = 0x5241_4431; // "RAD1"
const RECORD_SIZE: usize = 16;
const SECTOR_SIZE: u32 = 4096;

static CURRENT_DIVIDER_BITS: AtomicU32 = AtomicU32::new(DEFAULT_DIVIDER.to_bits());
static CURRENT_POLARITY_LOW: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
pub enum PulsePolarity {
    ActiveLow,
    ActiveHigh,
}

impl PulsePolarity {
    pub fn to_byte(self) -> u8 {
        match self {
            PulsePolarity::ActiveLow => 0,
            PulsePolarity::ActiveHigh => 1,
        }
    }

    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(PulsePolarity::ActiveLow),
            1 => Some(PulsePolarity::ActiveHigh),
            _ => None,
        }
    }

    pub fn is_low(self) -> bool {
        self == PulsePolarity::ActiveLow
    }
}

pub struct ConfigStore {
    flash: FlashStorage<'static>,
}

impl ConfigStore {
    pub fn new(flash: FLASH<'static>) -> Self {
        Self {
            flash: FlashStorage::new(flash),
        }
    }

    pub fn init(&mut self) -> (f32, PulsePolarity) {
        let (divider, polarity) = self.load().unwrap_or_else(|| {
            let _ = self.write(DEFAULT_DIVIDER, DEFAULT_POLARITY);
            (DEFAULT_DIVIDER, DEFAULT_POLARITY)
        });

        set_current_divider(divider);
        set_current_polarity(polarity);
        (divider, polarity)
    }

    pub fn divider(&self) -> f32 {
        current_divider()
    }

    pub fn polarity(&self) -> PulsePolarity {
        current_polarity()
    }

    pub fn save_divider(&mut self, divider: f32) -> Result<(), &'static str> {
        if !is_valid_divider(divider) {
            return Err("invalid divider");
        }
        self.write(divider, current_polarity())?;
        set_current_divider(divider);
        Ok(())
    }

    pub fn save_polarity(&mut self, polarity: PulsePolarity) -> Result<(), &'static str> {
        self.write(current_divider(), polarity)?;
        set_current_polarity(polarity);
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), &'static str> {
        self.write(DEFAULT_DIVIDER, DEFAULT_POLARITY)?;
        set_current_divider(DEFAULT_DIVIDER);
        set_current_polarity(DEFAULT_POLARITY);
        Ok(())
    }

    fn load(&mut self) -> Option<(f32, PulsePolarity)> {
        let addr = config_addr(&self.flash)?;
        let mut buf = [0u8; RECORD_SIZE];

        self.flash.read(addr, &mut buf).ok()?;
        decode(&buf)
    }

    fn write(&mut self, divider: f32, polarity: PulsePolarity) -> Result<(), &'static str> {
        let addr = config_addr(&self.flash).ok_or("flash too small")?;
        let buf = encode(divider, polarity);

        self.flash
            .erase(addr, addr + SECTOR_SIZE)
            .map_err(|_| "erase failed")?;

        self.flash.write(addr, &buf).map_err(|_| "write failed")?;

        Ok(())
    }
}

pub fn current_divider() -> f32 {
    f32::from_bits(CURRENT_DIVIDER_BITS.load(Ordering::Relaxed))
}

pub fn current_polarity() -> PulsePolarity {
    if CURRENT_POLARITY_LOW.load(Ordering::Relaxed) {
        PulsePolarity::ActiveLow
    } else {
        PulsePolarity::ActiveHigh
    }
}

pub fn is_valid_divider(divider: f32) -> bool {
    divider.is_finite() && divider > 0.0 && divider <= 1_000_000.0
}

fn set_current_divider(divider: f32) {
    CURRENT_DIVIDER_BITS.store(divider.to_bits(), Ordering::Relaxed);
}

fn set_current_polarity(polarity: PulsePolarity) {
    CURRENT_POLARITY_LOW.store(polarity.is_low(), Ordering::Relaxed);
}

fn config_addr(flash: &FlashStorage<'static>) -> Option<u32> {
    let capacity = flash.capacity();
    if capacity < SECTOR_SIZE as usize {
        return None;
    }
    Some((capacity - SECTOR_SIZE as usize) as u32)
}

fn encode(divider: f32, polarity: PulsePolarity) -> [u8; RECORD_SIZE] {
    let mut buf = [0u8; RECORD_SIZE];

    buf[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    buf[4..8].copy_from_slice(&divider.to_bits().to_le_bytes());
    buf[8] = polarity.to_byte();

    let crc = crc32(&buf[0..12]);
    buf[12..16].copy_from_slice(&crc.to_le_bytes());

    buf
}

fn decode(buf: &[u8; RECORD_SIZE]) -> Option<(f32, PulsePolarity)> {
    let magic = u32::from_le_bytes(buf[0..4].try_into().ok()?);
    let bits = u32::from_le_bytes(buf[4..8].try_into().ok()?);
    let polarity_byte = buf[8];
    let crc = u32::from_le_bytes(buf[12..16].try_into().ok()?);

    if magic != MAGIC {
        return None;
    }

    if crc != crc32(&buf[0..12]) {
        return None;
    }

    let divider = f32::from_bits(bits);
    if !is_valid_divider(divider) {
        return None;
    }

    let polarity = PulsePolarity::from_byte(polarity_byte)?;
    Some((divider, polarity))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
