use crate::{
    codec::{Reader, Writer},
    error::ProtocolResult,
};

pub trait Field: Sized {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self>;

    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult;
}

macro_rules! numeric_fields {
    ($($ty:ty),* $(,)?) => {$(
        impl Field for $ty {
            fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
                reader.array().map(<$ty>::from_le_bytes)
            }

            fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
                writer.raw(&self.to_le_bytes());
                Ok(())
            }
        }
    )*};
}

numeric_fields!(u8, i8, u16, i16, u32, i32, u64, f32);

impl Field for bool {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        Ok(reader.u8()? != 0)
    }

    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.u8(u8::from(*self));
        Ok(())
    }
}

impl Field for String {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        reader.string().map(str::to_owned)
    }

    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.string(self)
    }
}

impl<T: Field + Copy + Default, const N: usize> Field for [T; N] {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        let mut items = [T::default(); N];
        for item in &mut items {
            *item = T::decode(reader)?;
        }
        Ok(items)
    }

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

impl Field for Color {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        let [r, g, b] = reader.array::<3>()?;
        Ok(Self { r, g, b })
    }

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

impl Field for Buff {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        Ok(Self { id: u16::decode(reader)?, time: i16::decode(reader)? })
    }

    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        self.id.encode(writer)?;
        self.time.encode(writer)
    }
}
