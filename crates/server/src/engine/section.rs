mod cache;
mod encode;

use std::borrow::Cow;
use std::mem;
use std::ops::{Index, IndexMut, Range};

use cobalt_format::world::header::Size;
use cobalt_protocol::NetworkText;
use cobalt_protocol::registry::{
    CompleteConnectionAndSpawn, PlayerControls, SectionTileFrame, Status,
};

use super::error::ClientError;
use super::world_info::narrow;
use super::{Outbox, State};
use crate::model::ClientId;

pub(super) use cache::Cache;

pub(super) const TILE_PIXELS: f32 = 16.0;
const SECTION_WIDTH: usize = 200;
const SECTION_HEIGHT: usize = 150;
const RADIUS_X: usize = 2;
const RADIUS_Y: usize = 1;
const STATUS_TEXT: &str = "LegacyInterface.44";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Section {
    column: usize,
    row: usize,
}

impl Section {
    fn containing(size: Size, x: i32, y: i32) -> Option<Self> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        (x < size.width && y < size.height)
            .then_some(Self { column: x / SECTION_WIDTH, row: y / SECTION_HEIGHT })
    }

    fn region(self, size: Size) -> Region {
        let (x, y) = (self.column * SECTION_WIDTH, self.row * SECTION_HEIGHT);
        Region {
            columns: x..size.width.min(x + SECTION_WIDTH),
            rows: y..size.height.min(y + SECTION_HEIGHT),
        }
    }
}

struct Region {
    columns: Range<usize>,
    rows: Range<usize>,
}

impl Region {
    fn contains(&self, x: i32, y: i32) -> bool {
        within(&self.columns, x) && within(&self.rows, y)
    }
}

pub(super) struct Atlas<T> {
    columns: usize,
    cells: Box<[T]>,
}

impl<T: Clone> Atlas<T> {
    fn new(size: Size, fill: T) -> Self {
        let columns = size.width.div_ceil(SECTION_WIDTH);
        let rows = size.height.div_ceil(SECTION_HEIGHT);
        Self { columns, cells: vec![fill; columns * rows].into_boxed_slice() }
    }
}

impl<T> Atlas<T> {
    fn sections(&self) -> impl Iterator<Item = Section> {
        let columns = self.columns;
        (0..self.cells.len())
            .map(move |index| Section { column: index % columns, row: index / columns })
    }
}

impl<T> Index<Section> for Atlas<T> {
    type Output = T;

    fn index(&self, section: Section) -> &T {
        &self.cells[section.row * self.columns + section.column]
    }
}

impl<T> IndexMut<Section> for Atlas<T> {
    fn index_mut(&mut self, section: Section) -> &mut T {
        &mut self.cells[section.row * self.columns + section.column]
    }
}

pub(super) type Streamed = Atlas<bool>;

impl Streamed {
    pub(super) fn unsent(size: Size) -> Self {
        Self::new(size, false)
    }

    fn reset(&mut self) {
        self.cells.fill(false);
    }

    fn claim(&mut self, section: Section) -> bool {
        !mem::replace(&mut self[section], true)
    }
}

struct Window {
    columns: Range<usize>,
    rows: Range<usize>,
}

impl Window {
    fn around(size: Size, x: i32, y: i32) -> Self {
        Self {
            columns: window(
                narrow::<i32, usize>(x) / SECTION_WIDTH,
                RADIUS_X,
                size.width.div_ceil(SECTION_WIDTH),
            ),
            rows: window(
                narrow::<i32, usize>(y) / SECTION_HEIGHT,
                RADIUS_Y,
                size.height.div_ceil(SECTION_HEIGHT),
            ),
        }
    }

    fn len(&self) -> usize {
        self.columns.len() * self.rows.len()
    }

    fn sections(&self) -> impl Iterator<Item = Section> {
        self.columns
            .clone()
            .flat_map(|column| self.rows.clone().map(move |row| Section { column, row }))
    }
}

impl State {
    pub(super) fn stream_sections(
        &mut self,
        id: ClientId,
        controls: &PlayerControls<'_>,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let size = self.world.header.size;
        let window = Window::around(size, tile(controls.x), tile(controls.y));
        self.stream(id, &window, out)
    }

    pub(super) fn stream_spawn(
        &mut self,
        id: ClientId,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let spawn = self.world.header.locations.spawn;
        let window = Window::around(self.world.header.size, spawn.x, spawn.y);

        let status = Status {
            status_max: narrow(window.len()),
            text: NetworkText::Localized { key: STATUS_TEXT, args: Cow::Borrowed(&[]) },
            flags: 0,
        };
        out.send(id, status.into())?;
        self.streams[id.index()].reset();
        self.stream(id, &window, out)?;
        out.send(id, CompleteConnectionAndSpawn {}.into())?;
        Ok(())
    }

    pub(super) fn invalidate_section(&mut self, x: i32, y: i32) {
        if let Some(section) = Section::containing(self.world.header.size, x, y) {
            self.sections.invalidate(section);
        }
    }

    fn stream(
        &mut self,
        id: ClientId,
        window: &Window,
        out: &mut Outbox,
    ) -> Result<(), ClientError> {
        let streamed = &mut self.streams[id.index()];
        let pending: Vec<Section> =
            window.sections().filter(|&section| streamed.claim(section)).collect();
        let Some(frame) = frame(&pending) else { return Ok(()) };

        self.sections.ensure(&self.world, &pending)?;
        for &section in &pending {
            if let Some(packet) = self.sections.packet(section) {
                out.relay(id, packet);
            }
        }
        out.send(id, frame.into())?;
        Ok(())
    }
}

fn frame(sections: &[Section]) -> Option<SectionTileFrame> {
    let columns = sections.iter().map(|section| section.column);
    let rows = sections.iter().map(|section| section.row);
    Some(SectionTileFrame {
        start_x: narrow(columns.clone().min()?),
        start_y: narrow(rows.clone().min()?),
        end_x: narrow(columns.max()?),
        end_y: narrow(rows.max()?),
    })
}

fn tile(pixels: f32) -> i32 {
    (pixels / TILE_PIXELS) as i32
}

fn window(center: usize, radius: usize, count: usize) -> Range<usize> {
    let center = center.min(count.saturating_sub(1));
    center.saturating_sub(radius)..count.min(center + radius + 1)
}

fn within(range: &Range<usize>, value: i32) -> bool {
    usize::try_from(value).is_ok_and(|value| range.contains(&value))
}
