use chrono::{Date, Datelike, Duration, TimeZone, Utc};
use core::convert::TryFrom;
use core::fmt;
use rand::{thread_rng, Rng};
use std::ops::{Add, BitAnd, Mul, Shr};
use std::string::String;
use strum_macros::EnumIter;
use thiserror::Error;

const KEYS_SIZE: i32 = KEY_CHARS.len() as i32;
const KEY_CHARS: [u8; 34] = [
    b'D', b'Y', b'1', b'4', b'U', b'F', b'3', b'R', b'H', b'W', b'C', b'X', b'L', b'Q', b'B', b'6',
    b'I', b'K', b'J', b'T', b'9', b'N', b'5', b'A', b'G', b'S', b'2', b'P', b'M', b'8', b'V', b'Z',
    b'7', b'E',
];

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("key has an invalid checksum")]
    InvalidChecksum { expected: u16, found: u16 },
    #[error("key has an invalid length")]
    InvalidLength { expected: usize, found: usize },
    #[error("key belongs to an unknown edition")]
    UnknownEdition,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, EnumIter)]
pub enum KeyEdition {
    Business = 0,
    Extreme = 1,
    Engineer = 2,
    NetworkAudit = 3,
}

impl fmt::Display for KeyEdition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KeyEdition::Business => write!(f, "Business"),
            KeyEdition::Extreme => write!(f, "Extreme"),
            KeyEdition::Engineer => write!(f, "Engineer"),
            KeyEdition::NetworkAudit => write!(f, "Network Audit"),
        }
    }
}

impl TryFrom<i32> for KeyEdition {
    type Error = KeyError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(KeyEdition::Business),
            1 => Ok(KeyEdition::Extreme),
            2 => Ok(KeyEdition::Engineer),
            3 => Ok(KeyEdition::NetworkAudit),
            _ => Err(KeyError::UnknownEdition),
        }
    }
}

impl TryFrom<&str> for KeyEdition {
    type Error = KeyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "business" => Ok(KeyEdition::Business),
            "extreme" => Ok(KeyEdition::Extreme),
            "engineer" => Ok(KeyEdition::Engineer),
            "network" => Ok(KeyEdition::NetworkAudit),
            _ => Err(KeyError::UnknownEdition),
        }
    }
}

trait DateExt {
    fn enc(&self) -> i32;
    fn dec(val: i32) -> Date<Utc>;
}

impl DateExt for Date<Utc> {
    fn enc(&self) -> i32 {
        let year = self.year().clamp(2004, 2099) - 2003;
        let month = self.month().clamp(1, 12);
        let day = self.day().clamp(1, 31);
        year.mul(512).add(month.mul(32).add(day) as i32)
    }

    fn dec(val: i32) -> Date<Utc> {
        let day = val.bitand(31) as u32;
        let month = val.shr(5u32).bitand(15) as u32;
        let year = val.shr(9u32).bitand(31).add(2003);
        Utc.ymd(year, month, day)
    }
}

#[derive(Debug, Clone)]
pub struct License {
    pub edition: KeyEdition,
    pub seats: i32,
    pub purchase_date: Date<Utc>,
    pub expiry: Option<Duration>,
    pub maintenance_expiry: Duration,

    unk1: i32,
    unk2: i32,
    unk3: i32,
}

impl License {
    pub fn new(edition: KeyEdition) -> License {
        let mut rng = thread_rng();

        let unk1: i32 = rng.gen_range(100, 989);
        let unk2: i32 = rng.gen_range(0, 100);
        let unk3: i32 = rng.gen_range(0, 100);

        License {
            edition,
            purchase_date: Utc::today(),
            expiry: None,
            seats: 1,
            maintenance_expiry: Duration::days(3658),

            unk1,
            unk2,
            unk3,
        }
    }

    pub fn with_purchase_date(mut self, date: Date<Utc>) -> Self {
        let date_2004 = Utc.ymd(2004, 1, 1);
        let date_2099 = Utc.ymd(2099, 1, 1);
        self.purchase_date = date.clamp(date_2004, date_2099);
        self
    }

    pub fn with_edition(mut self, edition: KeyEdition) -> Self {
        self.edition = edition;
        self
    }

    pub fn with_seats(mut self, seats: i32) -> Self {
        self.seats = seats.clamp(1, 797);
        self
    }

    pub fn with_license_expiry(mut self, duration: Option<Duration>) -> Self {
        self.expiry = duration;
        self
    }

    pub fn with_maintenance_expiry(mut self, duration: Duration) -> Self {
        self.maintenance_expiry = duration.clamp(Duration::days(1), Duration::days(3658));
        self
    }

    pub fn from_key<T: AsRef<[u8]>>(key: T) -> Result<License, KeyError> {
        let key =
            key.as_ref().iter().filter(|b| b.is_ascii_alphanumeric()).copied().collect::<Vec<u8>>();

        if key.len() != 25 {
            return Err(KeyError::InvalidLength { expected: 25, found: key.len() });
        }

        if !verify_checksum(&key) {
            return Err(KeyError::InvalidChecksum {
                expected: get_checksum(&key[0..24]),
                found: key.last().copied().unwrap() as u16,
            });
        }

        let key_parts: [i32; 9] = [
            dec_part(&key[0..2]),
            dec_part(&key[2..4]),
            dec_part(&key[4..6]),
            dec_part(&key[6..8]),
            dec_part(&key[8..12]),
            dec_part(&key[12..16]),
            dec_part(&key[16..19]),
            dec_part(&key[19..22]),
            dec_part(&key[22..24]),
        ];

        let edition = ((key_parts[8] & 0xFF) ^ key_parts[0] ^ 0xBF) - 1;
        let edition = KeyEdition::try_from(edition)?;

        let seats = key_parts[8] ^ key_parts[4] ^ 0x4755;
        let purchase_date = Date::dec(key_parts[8] ^ key_parts[5] ^ 0x7CC1);

        let expiry = (key_parts[8] & 0xFF) ^ key_parts[6] ^ 0x3FD;
        let expiry = match expiry {
            0 => None,
            _ => Some(Date::dec(expiry) - purchase_date),
        };

        let maintenance_expiry = (key_parts[8] & 0xFF) ^ key_parts[7] ^ 0x935;
        let maintenance_expiry = Duration::days(maintenance_expiry as i64);

        let unk1 = (key_parts[8] & 0xFF) ^ key_parts[1] ^ 0xED;
        let unk2 = (key_parts[8] & 0xFF) ^ (key_parts[2] & 0xFFFF) ^ 0x77;
        let unk3 = (key_parts[8] & 0xFF) ^ (key_parts[3] & 0xFFFF) ^ 0xDF;

        Ok(License { edition, seats, purchase_date, expiry, maintenance_expiry, unk1, unk2, unk3 })
    }

    pub fn generate(&self) -> [u8; 25] {
        let mut enc_key: [u8; 25] = [0; 25];
        gen_pair(&mut enc_key[22..24]);

        let purchase_date = self.purchase_date.enc();
        let expiry = self.expiry.map(|exp| exp.num_days()).unwrap_or(0) as i32;
        let maintenance_expiry = self.maintenance_expiry.num_days() as i32;

        let base_val = dec_part(&mut enc_key[22..24]);
        enc_part((base_val & 0xFF) ^ (self.edition as i32 + 1) ^ 0xBF, &mut enc_key[0..2]);
        enc_part((base_val & 0xFF) ^ self.unk1 ^ 0xED, &mut enc_key[2..4]);
        enc_part((base_val & 0xFF) ^ self.unk2 ^ 0x77, &mut enc_key[4..6]);
        enc_part((base_val & 0xFF) ^ self.unk3 ^ 0xDF, &mut enc_key[6..8]);
        enc_part((base_val & 0xFFFFFF) ^ self.seats ^ 0x4755, &mut enc_key[8..12]);
        enc_part((base_val & 0xFFFFFF) ^ purchase_date ^ 0x7CC1, &mut enc_key[12..16]);
        enc_part((base_val & 0xFF) ^ expiry ^ 0x3FD, &mut enc_key[16..19]);
        enc_part((base_val & 0xFF) ^ maintenance_expiry ^ 0x935, &mut enc_key[19..22]);

        let mut enc_checksum: [u8; 3] = [0; 3];
        enc_part(get_checksum(&mut enc_key[0..24]) as i32, &mut enc_checksum);

        enc_key[24] = enc_checksum[1];
        enc_key
    }

    pub fn generate_string(&self, separators: bool) -> String {
        let mut key = self.generate().to_vec();

        if separators {
            key.insert(20, b'-');
            key.insert(15, b'-');
            key.insert(10, b'-');
            key.insert(5, b'-');
        }

        String::from_utf8(key).unwrap()
    }

    pub fn is_valid_key(&self) -> bool {
        let mut days_left = 0;

        let date_2004 = Utc.ymd(2004, 1, 1);
        let date_2099 = Utc.ymd(2099, 1, 1);

        if (date_2004..=date_2099).contains(&self.purchase_date) {
            let current_days = Utc::today().enc();
            let purchase_days = self.purchase_date.enc();
            let expiry_days = self.expiry.map(|exp| exp.num_days()).unwrap_or(0) as i32;
            days_left = (expiry_days + purchase_days) - current_days
        }

        (self.expiry.is_none() || days_left > 0)
            && (0..797).contains(&self.seats)
            && (99..990).contains(&self.unk1)
            && self.unk2 <= 100
            && self.unk3 <= 100
            && self.maintenance_expiry.num_days() < 3659
    }
}

fn gen_pair(slice: &mut [u8]) {
    slice.iter_mut().for_each(|x| *x = KEY_CHARS[thread_rng().gen_range(0, KEYS_SIZE) as usize])
}

fn enc_part(mut val: i32, slice: &mut [u8]) {
    slice.iter_mut().rev().for_each(|x| {
        *x = KEY_CHARS[(val % KEYS_SIZE) as usize];
        val /= KEYS_SIZE;
    })
}

fn dec_part<T: AsRef<[u8]>>(key_part: T) -> i32 {
    key_part.as_ref().iter().fold(0i32, |result, c1| {
        (result * KEYS_SIZE) + KEY_CHARS.iter().position(|&c2| c2 == *c1).unwrap_or(0) as i32
    })
}

fn get_checksum<T: AsRef<[u8]>>(key_part: T) -> u16 {
    let checksum = (key_part.as_ref().iter().fold(0u32, |result, b| {
        (0..8).fold(result ^ (*b as u32) << 8, |result, _| {
            if result & 0x8000 == 0 {
                result << 1
            } else {
                result << 1 ^ 0x8201
            }
        })
    }) & 0xFFFF) as u16;

    checksum % 0x9987
}

fn verify_checksum<T: AsRef<[u8]>>(key: T) -> bool {
    let key = key.as_ref();
    key.len() == 25 && {
        let mut enc_checksum: [u8; 3] = [0; 3];
        enc_part(get_checksum(&key[0..24]) as i32, &mut enc_checksum);

        enc_checksum[1] == key[24]
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn parse_license() {
        assert!(
            License::from_key("  3BH41-94ZD6 4KDT5JD-PUY_TBSN9 ").unwrap().is_valid_key(),
            "parsed valid license as invalid!"
        );

        assert!(
            License::from_key("  3BH41-94ZD6 4KDT5JD-PUY_TBSN2 ").is_err(),
            "parsed license did not trip the checksum check!"
        );
    }

    #[test]
    fn generate() {
        for edition in KeyEdition::iter() {
            assert!(License::new(edition).is_valid_key(), "generated invalid license!");
            assert!(
                License::new(edition).with_license_expiry(Some(Duration::days(50))).is_valid_key(),
                "generated invalid license when using an expiry!"
            );
        }
    }
}
