use derive_more::derive::Constructor;
use enumflags2::{BitFlag, BitFlags, bitflags};
use getset::{Getters, Setters};

use crate::{common::codec::RLEBinaryDecode, io::BinaryReader, util::coordinates::UVCoordinates, world::{frame_important::WorldFrameImportant, tiles::{error::WorldTilesError, tile::{block::{Block, BlockShape}, liquid::{Liquid, LiquidKind}, mechanicals::{MechanicalState, Mechanicals}, paint::{BlockPaint, WallPaint}, wall::Wall}}}};

pub mod block;
pub mod liquid;
pub mod wall;
pub mod mechanicals;
pub mod paint;

#[bitflags]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Flag1 {
    Flag2Exists, // 1
    BlockId1,    // 2
    WallId1,     // 4
    LiquidId1,   // 8
    LiquidId2,   // 16
    BlockId2,    // 32
    Rle1,        // 64
    Rle2,        // 128
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Flag2 {
    Flag3Exists, // 1
    IsRedWire,   // 2
    IsBlueWire,  // 4
    IsGreenWire, // 8
    SlopeId1,    // 16
    SlopeId2,    // 32
    SlopeId3,    // 64
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Flag3 {
    Flag4Exists,     // 1
    IsActuator,      // 2
    IsActuated,      // 4
    BlockColorId,    // 8
    WallColorId,     // 16
    IsYellowWire,    // 32
    WallId2,         // 64
    IsShimmerLiquid, // 128
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Flag4 {
    IsBlockEcho       = 2,
    IsWallEcho        = 4,
    IsBlockIlluminant = 8,
    IsWallIlluminant  = 16,
}

#[derive(Debug, Constructor, Getters, Setters, PartialEq, Eq, Clone, Copy, Default)]
#[getset(get = "pub", set = "pub")]
pub struct Tile {
    block: Option<Block>,
    wall: Option<Wall>,
    liquid: Option<Liquid>,
    mechanicals: Mechanicals,
}

impl RLEBinaryDecode for Tile {
    type Error = WorldTilesError;
    type Context<'a> = WorldFrameImportant;

    fn rle_decode<R: BinaryReader>(
        ctx: Option<&Self::Context<'_>>,
        reader: &mut R
    ) -> Result<(Self, usize), Self::Error> {
        let ctx = ctx.ok_or_else(|| anyhow::anyhow!("Failed to decode Tile: context was None"))?;

        let flags1 = Flag1::from_bits_truncate(reader.u8()?);

        let flags2 = flags1.contains(Flag1::Flag2Exists)
            .then(|| reader.u8())// returns Option<Result<u8, Error>>
            .transpose()?
            .map(Flag2::from_bits_truncate);

        let flags3 = flags2
            .and_then(|f| f.contains(Flag2::Flag3Exists).then(|| reader.u8()))
            .transpose()?
            .map(Flag3::from_bits_truncate);

        let flags4 = flags3
            .and_then(|f| f.contains(Flag3::Flag4Exists).then(|| reader.u8()))
            .transpose()?
            .map(Flag4::from_bits_truncate);

        let f2 = flags2.unwrap_or_else(Flag2::empty);
        let f3 = flags3.unwrap_or_else(Flag3::empty);
        let f4 = flags4.unwrap_or_else(Flag4::empty);

        // Block
        let block = if flags1.contains(Flag1::BlockId1) {
            let block_id = if flags1.contains(Flag1::BlockId2) {
                reader.u16()?
            } else {
                reader.u8()? as u16
            };

            let block_uv = ctx.get(block_id as usize)
                .copied()
                .unwrap_or(false)
                .then(|| -> Result<_, Self::Error> {
                    let u = reader.i16()?;
                    let v = reader.i16()?;
                    Ok(UVCoordinates::new_uv(u, if block_id == 144 { 0 } else { v }))
                })
                .transpose()?;

            let block_paint = if f3.contains(Flag3::BlockColorId) {
                BlockPaint::from(reader.u8()?)
            } else {
                BlockPaint::Unpainted
            };

            let n_9 = (f2 & (Flag2::SlopeId1 | Flag2::SlopeId2 | Flag2::SlopeId3)).bits() >> 3;
            let block_shape = match n_9 {
                0 => BlockShape::Normal,
                1 => BlockShape::HalfBrick,
                _ => BlockShape::from(n_9 - 1),
            };

            let block_actuated = f3.contains(Flag3::IsActuated);
            let block_illuminant = f4.contains(Flag4::IsBlockIlluminant);
            let block_echo = f4.contains(Flag4::IsBlockEcho);

            Some(Block::new(block_id, block_uv, block_shape, block_actuated, block_illuminant, block_echo, block_paint))
        } else {
            None
        };

        // Wall
        let wall = if flags1.contains(Flag1::WallId1) {
            let mut wall_id = reader.u8()? as u16;
            
            if f3.contains(Flag3::WallId2) {
                wall_id |= (reader.u8()? as u16) << 8;
            }

            let wall_paint = if f3.contains(Flag3::WallColorId) {
                WallPaint::from(reader.u8()? as u16)
            } else {
                WallPaint::Unpainted
            };

            Some(Wall::new(
                wall_id, 
                f4.contains(Flag4::IsWallIlluminant), 
                f4.contains(Flag4::IsWallEcho), 
                wall_paint
            ))
        } else {
            None
        };

        // Liquid
        let liquid_bits = (flags1 & (Flag1::LiquidId1 | Flag1::LiquidId2)).bits() >> 3;
        let liquid = if liquid_bits > 0 {
            let liquid_kind = if f3.contains(Flag3::IsShimmerLiquid) {
                LiquidKind::Shimmer
            } else {
                LiquidKind::from(liquid_bits)
            };

            Some(Liquid::new(liquid_kind, reader.u8()?))
        } else {
            None
        };

        // Mechanicals
        let mut mechanicals = BitFlags::empty();
        mechanicals.set(MechanicalState::HasRedWire, f2.contains(Flag2::IsRedWire));
        mechanicals.set(MechanicalState::HasGreenWire, f2.contains(Flag2::IsGreenWire));
        mechanicals.set(MechanicalState::HasBlueWire, f2.contains(Flag2::IsBlueWire));
        mechanicals.set(MechanicalState::HasYellowWire, f3.contains(Flag3::IsYellowWire));
        mechanicals.set(MechanicalState::HasActuator, f3.contains(Flag3::IsActuator));

        let repeat_count = match flags1 & (Flag1::Rle1 | Flag1::Rle2) {
            f if f == Flag1::Rle1 => reader.u8()? as usize,
            f if f == Flag1::Rle2 => reader.u16()? as usize,
            _ => 0,
        };

        Ok((Self { block, wall, liquid, mechanicals }, repeat_count))
    }
}

