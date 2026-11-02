use crate::{
    error::{Error, Result},
    reader::Reader,
    world::header::Header,
};

pub fn verify(reader: &mut Reader<'_>, header: &Header) -> Result {
    let valid = reader.bool()? && reader.string()? == header.name && reader.i32()? == header.id;
    if valid {
        Ok(())
    } else {
        Err(Error::InvalidFooter)
    }
}
