use std::borrow::Cow;
use std::io::{self, Write};
use std::ops::Range;

use cobalt_format::World;
use cobalt_format::world::header::Size;
use cobalt_protocol::registry::{CompleteConnectionAndSpawn, Status, TileSection};
use cobalt_protocol::{NetworkText, ProtocolResult, Remainder, Writer};
use flate2::Compression;
use flate2::write::DeflateEncoder;

use super::Outbox;
use super::error::ClientError;
use super::world_info::narrow;
use crate::client::ClientId;

const SECTION_WIDTH: usize = 200;
const SECTION_HEIGHT: usize = 150;
const RADIUS_X: usize = 2;
const RADIUS_Y: usize = 1;
const STATUS_TEXT: &str = "LegacyInterface.44";

struct Section {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Section {
    fn new(size: Size, column: usize, row: usize) -> Self {
        let (x, y) = (column * SECTION_WIDTH, row * SECTION_HEIGHT);
        Self {
            x,
            y,
            width: SECTION_WIDTH.min(size.width - x),
            height: SECTION_HEIGHT.min(size.height - y),
        }
    }

    fn columns(&self) -> Range<usize> {
        self.x..self.x + self.width
    }

    fn rows(&self) -> Range<usize> {
        self.y..self.y + self.height
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        within(&self.columns(), x) && within(&self.rows(), y)
    }
}

pub(super) fn send_spawn(world: &World, to: ClientId, out: &mut Outbox) -> Result<(), ClientError> {
    let size = world.header.size;
    let spawn = world.header.locations.spawn;
    let columns = window(
        narrow::<i32, usize>(spawn.x) / SECTION_WIDTH,
        RADIUS_X,
        size.width.div_ceil(SECTION_WIDTH),
    );
    let rows = window(
        narrow::<i32, usize>(spawn.y) / SECTION_HEIGHT,
        RADIUS_Y,
        size.height.div_ceil(SECTION_HEIGHT),
    );

    let status = Status {
        status_max: narrow(columns.len() * rows.len()),
        text: NetworkText::Localized { key: STATUS_TEXT, args: Cow::Borrowed(&[]) },
        flags: 0,
    };
    out.send(to, status.into())?;

    let mut stream = Vec::new();
    for column in columns {
        for row in rows.clone() {
            stream.clear();
            write_stream(world, &Section::new(size, column, row), &mut stream)?;
            let payload = deflate(&stream)?;
            out.send(to, TileSection { payload: Remainder(&payload) }.into())?;
        }
    }

    out.send(to, CompleteConnectionAndSpawn {}.into())?;
    Ok(())
}

fn window(center: usize, radius: usize, count: usize) -> Range<usize> {
    let center = center.min(count.saturating_sub(1));
    center.saturating_sub(radius)..count.min(center + radius + 1)
}

fn within(range: &Range<usize>, value: i32) -> bool {
    usize::try_from(value).is_ok_and(|value| range.contains(&value))
}

fn deflate(stream: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder =
        DeflateEncoder::new(Vec::with_capacity(stream.len() / 2), Compression::default());
    encoder.write_all(stream)?;
    encoder.finish()
}

fn write_stream(world: &World, section: &Section, out: &mut Vec<u8>) -> ProtocolResult {
    out.extend_from_slice(&narrow::<usize, i32>(section.x).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i32>(section.y).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i16>(section.width).to_le_bytes());
    out.extend_from_slice(&narrow::<usize, i16>(section.height).to_le_bytes());

    write_tiles(world, section, out);
    write_labelled(
        out,
        section,
        world.chests.iter().map(|chest| (chest.x, chest.y, chest.name.as_str())),
    )?;
    write_labelled(
        out,
        section,
        world.signs.iter().map(|sign| (sign.x, sign.y, sign.text.as_str())),
    )?;
    out.extend_from_slice(&0i16.to_le_bytes());
    Ok(())
}

fn write_tiles(world: &World, section: &Section, out: &mut Vec<u8>) {
    let columns = section.columns();
    let mut tiles = section
        .rows()
        .flat_map(|y| columns.clone().map(move |x| (x, y)))
        .map(|(x, y)| world.tiles.get(x, y).copied().unwrap_or_default());

    let Some(mut current) = tiles.next() else { return };
    let mut run = 0u16;
    for tile in tiles {
        if tile == current && current.allows_batching() {
            run += 1;
        } else {
            current.write(out, &world.frame_important, run);
            current = tile;
            run = 0;
        }
    }
    current.write(out, &world.frame_important, run);
}

fn write_labelled<'a>(
    out: &mut Vec<u8>,
    section: &Section,
    entries: impl Iterator<Item = (i32, i32, &'a str)>,
) -> ProtocolResult {
    let entries: Vec<_> =
        entries.enumerate().filter(|(_, (x, y, _))| section.contains(*x, *y)).collect();

    out.extend_from_slice(&narrow::<usize, i16>(entries.len()).to_le_bytes());
    for (id, (x, y, label)) in entries {
        for value in [narrow::<usize, i16>(id), narrow(x), narrow(y)] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        Writer::new(out).string(label)?;
    }
    Ok(())
}
