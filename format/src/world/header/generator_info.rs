use derive_more::derive::Constructor;
use getset::{Getters, Setters};

#[derive(Debug, Constructor, Getters, Setters, PartialEq, Eq, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct GeneratorInfo {
    seed: String,
    version: u64,
}

/* impl BinaryEncode for GeneratorInfo {
    type Error = WorldSavingError;
    type Context<'a> = ();

    fn encode<W: BinaryWriter>(
        &self,
        _ctx: Option<&Self::Context<'_>>,
        writer: &mut W
    ) -> Result<(), Self::Error> {
        writer.string(&self.seed)
            .map_err(|_| WorldSavingError::WriteFailed("generator_info::seed"))?;

        writer.u64(self.version)
            .map_err(|_| WorldSavingError::WriteFailed("generator_info::version"))?;

        Ok(())
    }
} */

