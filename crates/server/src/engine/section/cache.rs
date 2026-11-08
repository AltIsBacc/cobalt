use cobalt_format::World;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{Atlas, Section, encode};
use crate::engine::error::ClientError;
use crate::log::warn_log;

pub(in crate::engine) struct Cache {
    packets: Atlas<Option<Box<[u8]>>>,
}

impl Cache {
    pub(in crate::engine) fn warmed(world: &World) -> Self {
        let mut cache = Self { packets: Atlas::new(world.header.size, None) };
        let sections: Vec<Section> = cache.packets.sections().collect();
        if let Err(error) = cache.ensure(world, &sections) {
            warn_log!("could not pre-encode every tile section: {error}");
        }
        cache
    }

    pub(super) fn packet(&self, section: Section) -> Option<&[u8]> {
        self.packets[section].as_deref()
    }

    pub(super) fn invalidate(&mut self, section: Section) {
        self.packets[section] = None;
    }

    pub(super) fn ensure(
        &mut self,
        world: &World,
        sections: &[Section],
    ) -> Result<(), ClientError> {
        let missing: Vec<Section> =
            sections.iter().copied().filter(|&section| self.packets[section].is_none()).collect();
        if missing.is_empty() {
            return Ok(());
        }

        let size = world.header.size;
        let encoded: Vec<_> = missing
            .par_iter()
            .map_init(Vec::new, |scratch, &section| {
                (section, encode::packet(world, &section.region(size), scratch))
            })
            .collect();

        let mut outcome = Ok(());
        for (section, packet) in encoded {
            match packet {
                Ok(packet) => self.packets[section] = Some(packet),
                Err(error) => outcome = outcome.and(Err(error)),
            }
        }
        outcome
    }
}
