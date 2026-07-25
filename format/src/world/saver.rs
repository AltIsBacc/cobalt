use std::{fs::OpenOptions, io::Write, ops::Deref, path::Path};

use derive_more::derive::{Deref, DerefMut};

use crate::{common::{Saver, codec::BinaryEncode}, io::writer::BinaryBuffer, world::{World, error::WorldSavingError}};

#[derive(Deref, DerefMut)]
pub struct WorldSaver<P: AsRef<Path>>(pub P);

impl<P: AsRef<Path>> Saver for WorldSaver<P> {
    type Input = World;
    type Error = WorldSavingError;

    fn save(&self, value: &Self::Input) -> Result<(), Self::Error> {
        let mut buffer = BinaryBuffer::new();

        value.metadata.encode(None, &mut buffer)?;
        value.sections.encode(None, &mut buffer)?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.deref())?;

        file.write_all(&buffer.into_inner())?;

        Ok(())
    }
}

