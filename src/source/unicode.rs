/// Represents all possible meanings for a given utf-8 byte and extracts the meaningful bits
///
/// > Do not construct this type directly, instead use `UTF8Byte::From(u8)` to ensure that
/// > variant accurately represents the data attached
pub enum UTF8Byte {
    OneByte(u8),
    TwoByte(u8),
    ThreeByte(u8),
    FourByte(u8),
    Continuation(u8),
    Invalid(u8),
}

impl From<u8> for UTF8Byte {
    fn from(value: u8) -> Self {
        match value {
            0b0000_0000..=0b0111_1111 => Self::OneByte(value),
            0b1000_0000..=0b1011_1111 => Self::Continuation(value & 0b0011_1111),
            0b1100_0000..=0b1101_1111 => Self::TwoByte(value & 0b0001_1111),
            0b1110_0000..=0b1110_1111 => Self::ThreeByte(value & 0b0000_1111),
            0b1111_0000..=0b1111_0111 => Self::FourByte(value & 0b0000_0111),
            _ => Self::Invalid(value),
        }
    }
}

impl From<UTF8Byte> for u8 {
    fn from(value: UTF8Byte) -> Self {
        match value {
            UTF8Byte::OneByte(v) => v & 0b0111_1111,
            UTF8Byte::TwoByte(v) => (v & 0b0001_1111) | 0b1100_0000,
            UTF8Byte::ThreeByte(v) => (v & 0b0000_1111) | 0b1110_0000,
            UTF8Byte::FourByte(v) => (v & 0b0000_0111) | 0b1000_0000,
            UTF8Byte::Continuation(v) => (v & 0b0011_1111) | 0b1000_0000,
            UTF8Byte::Invalid(v) => v,
        }
    }
}

