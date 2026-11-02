use crate::codec::{Reader, Writer};
use crate::error::ProtocolResult;

pub trait Decode<'data>: Sized {
    fn decode(reader: &mut Reader<'data>) -> ProtocolResult<Self>;
}

pub trait Encode {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult;
}

macro_rules! numeric_fields {
    ($($ty:ty),* $(,)?) => {$(
        impl Decode<'_> for $ty {
            fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
                reader.array().map(<$ty>::from_le_bytes)
            }
        }

        impl Encode for $ty {
            fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
                writer.raw(&self.to_le_bytes());
                Ok(())
            }
        }
    )*};
}

numeric_fields!(u8, i8, u16, i16, u32, i32, u64, f32);

impl Decode<'_> for bool {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        Ok(reader.u8()? != 0)
    }
}

impl Encode for bool {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.u8(u8::from(*self));
        Ok(())
    }
}

impl<'data> Decode<'data> for &'data str {
    fn decode(reader: &mut Reader<'data>) -> ProtocolResult<Self> {
        reader.string()
    }
}

impl Encode for &str {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.string(self)
    }
}

impl<'data, T: Decode<'data> + Copy + Default, const N: usize> Decode<'data> for [T; N] {
    fn decode(reader: &mut Reader<'data>) -> ProtocolResult<Self> {
        let mut items = [T::default(); N];
        for item in &mut items {
            *item = T::decode(reader)?;
        }
        Ok(items)
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        self.iter().try_for_each(|item| item.encode(writer))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Decode<'_> for Color {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        let [r, g, b] = reader.array::<3>()?;
        Ok(Self { r, g, b })
    }
}

impl Encode for Color {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.raw(&[self.r, self.g, self.b]);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buff {
    pub id: u16,
    pub time: i16,
}

impl Decode<'_> for Buff {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        Ok(Self { id: u16::decode(reader)?, time: i16::decode(reader)? })
    }
}

impl Encode for Buff {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        self.id.encode(writer)?;
        self.time.encode(writer)
    }
}
