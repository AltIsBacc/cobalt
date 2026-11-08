use std::io::{self, Write};

use cobalt_format::World;
use cobalt_protocol::registry::TileSection;
use cobalt_protocol::{HEADER_LEN, ProtocolResult, Remainder, S2c, Writer};
use flate2::Compression;
use flate2::write::DeflateEncoder;

use super::Region;
use crate::engine::error::ClientError;
use crate::engine::world_info::narrow;

pub(super) fn packet(
    world: &World,
    region: &Region,
    scratch: &mut Vec<u8>,
) -> Result<Box<[u8]>, ClientError> {
    scratch.clear();
    write_stream(world, region, scratch)?;
    let payload = deflate(scratch)?;

    let mut packet = Vec::with_capacity(HEADER_LEN + payload.len());
    S2c::from(TileSection { payload: Remainder(&payload) }).write(&mut packet)?;
    Ok(packet.into_boxed_slice())
}

fn deflate(stream: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder =
        DeflateEncoder::new(Vec::with_capacity(stream.len() / 2), Compression::default());
    encoder.write_all(stream)?;
    encoder.finish()
}

fn write_stream(world: &World, region: &Region, out: &mut Vec<u8>) -> ProtocolResult {
    out.extend_from_slice(&narrow::<usize, i32>(region.columns.start).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i32>(region.rows.start).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i16>(region.columns.len()).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i16>(region.rows.len()).to_le_bytes());

    world.tiles.write_region(
        region.columns.clone(),
        region.rows.clone(),
        &world.frame_important,
        out,
    );
    write_labelled(
        out,
        region,
        world.chests.iter().map(|chest| (chest.x, chest.y, chest.name.as_str())),
    )?;
    write_labelled(
        out,
        region,
        world.signs.iter().map(|sign| (sign.x, sign.y, sign.text.as_str())),
    )?;
    out.extend_from_slice(&0i16.to_le_bytes());
    Ok(())
}

fn write_labelled<'a>(
    out: &mut Vec<u8>,
    region: &Region,
    entries: impl Iterator<Item = (i32, i32, &'a str)>,
) -> ProtocolResult {
    let entries: Vec<_> =
        entries.enumerate().filter(|(_, (x, y, _))| region.contains(*x, *y)).collect();

    out.extend_from_slice(&narrow::<usize, i16>(entries.len()).to_le_bytes());
    for (id, (x, y, label)) in entries {
        for value in [narrow::<usize, i16>(id), narrow(x), narrow(y)] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        Writer::new(out).string(label)?;
    }
    Ok(())
}
