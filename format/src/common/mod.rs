
pub mod codec;
pub mod metadata;

pub trait Loader {
    type Output;
    type Error;

    fn load(&self) -> Result<Self::Output, Self::Error>;
}

pub trait Saver {
    type Input;
    type Error;

    fn save(&self, value: &Self::Input) -> Result<(), Self::Error>;
}

