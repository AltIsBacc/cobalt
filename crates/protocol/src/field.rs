use crate::codec::{Reader, Writer};
use crate::error::{ProtocolError, ProtocolResult};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Remainder<'data>(pub &'data [u8]);

impl<'data> Decode<'data> for Remainder<'data> {
    fn decode(reader: &mut Reader<'data>) -> ProtocolResult<Self> {
        Ok(Self(reader.rest()))
    }
}

impl Encode for Remainder<'_> {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        writer.raw(self.0);
        Ok(())
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

pub const MAX_BUFFS: usize = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuffList(pub [u16; MAX_BUFFS]);

impl Default for BuffList {
    fn default() -> Self {
        Self([0; MAX_BUFFS])
    }
}

impl Decode<'_> for BuffList {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        let mut ids = [0; MAX_BUFFS];
        for id in &mut ids {
            match u16::decode(reader)? {
                0 => break,
                value => *id = value,
            }
        }
        Ok(Self(ids))
    }
}

impl Encode for BuffList {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        let mut written = 0;
        for id in self.0.iter().take_while(|&&id| id != 0) {
            id.encode(writer)?;
            written += 1;
        }
        if written < MAX_BUFFS {
            0u16.encode(writer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpawnPoint {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpawnPoints(pub Vec<SpawnPoint>);

impl Decode<'_> for SpawnPoints {
    fn decode(reader: &mut Reader<'_>) -> ProtocolResult<Self> {
        let count = usize::from(reader.u8()?);
        let mut points = Vec::with_capacity(count);
        for _ in 0..count {
            points.push(SpawnPoint { x: i16::decode(reader)?, y: i16::decode(reader)? });
        }
        Ok(Self(points))
    }
}

impl Encode for SpawnPoints {
    fn encode(&self, writer: &mut Writer<'_>) -> ProtocolResult {
        let count = u8::try_from(self.0.len())
            .map_err(|_| ProtocolError::TooManySpawnPoints(self.0.len()))?;
        writer.u8(count);
        for point in &self.0 {
            point.x.encode(writer)?;
            point.y.encode(writer)?;
        }
        Ok(())
    }
}
