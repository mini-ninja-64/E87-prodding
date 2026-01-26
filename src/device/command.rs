const SET_LANGUAGE: u8 = 0x16;
const REQUEST_PICTURE_SIZE: u8 = 0xDA;

pub const REQUEST_BADGE_INFO: u8 = 0xC6;
const RESPONSE_BADGE_INFO: u8 = 0xC7;

#[repr(u8)]
pub enum RequestCommand {
    BadgeInfo = 0xC6,
    PictureSize = 0xDA,
}

impl Into<u8> for RequestCommand {
    fn into(self) -> u8 {
        self as u8
    }
}
