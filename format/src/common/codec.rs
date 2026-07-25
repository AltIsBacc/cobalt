use crate::io::{BinaryReader, BinaryWriter};
 
pub trait BinaryDecode: Sized {
    type Error;
    type Context<'a>;

    fn decode<R: BinaryReader>(ctx: Option<&Self::Context<'_>>, reader: &mut R) -> Result<Self, Self::Error>;
}

pub trait BinaryEncode {
    type Error;
    type Context<'a>;

    fn encode<W: BinaryWriter>(&self, ctx: Option<&Self::Context<'_>>, writer: &mut W) -> Result<(), Self::Error>;
}

pub trait RLEBinaryDecode: Sized {
    type Error;
    type Context<'a>;

    fn rle_decode<R: BinaryReader>(ctx: Option<&Self::Context<'_>>, reader: &mut R) -> Result<(Self, usize), Self::Error>;
}

pub trait RLEBinaryEncode {
    type Error;
    type Context<'a>;

    fn rle_encode<W: BinaryWriter>(&self, ctx: Option<&Self::Context<'_>>, writer: &mut W, repeat: usize) -> Result<(), Self::Error>;
}

