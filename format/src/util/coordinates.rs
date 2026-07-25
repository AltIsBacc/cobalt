use derive_more::{Add, AddAssign, Constructor, Display, From, Into, Mul, MulAssign, Neg, Sub, SubAssign};
use getset::{CopyGetters, MutGetters, Setters};

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
    Constructor,
    From, Into,
    Add, AddAssign, Sub, SubAssign,
    Mul, MulAssign,
    Neg,
    Display,
    CopyGetters, MutGetters, Setters,
)]
#[display("({}, {})", x, y)]
pub struct Coordinates<XT: Copy = i32, YT: Copy = i32> {
    #[getset(get_copy = "pub", get_mut = "pub", set = "pub")]
    x: XT,

    #[getset(get_copy = "pub", get_mut = "pub", set = "pub")]
    y: YT,
}

impl<XT: Copy, YT: Copy> Coordinates<XT, YT> {
    pub const fn new_rev(y: YT, x: XT) -> Self {
        Self::new(x, y)
    }

    pub fn new_xy(x: XT, y: YT) -> Self {
        Self::new(x, y)
    }
    pub fn new_uv(u: XT, v: YT) -> Self {
        Self::new(u, v)
    }
    pub fn new_size(width: XT, height: YT) -> Self {
        Self::new(width, height)
    }

    pub fn u(&self) -> XT {
        self.x()
    }
    pub fn v(&self) -> YT {
        self.y()
    }
    pub fn width(&self) -> XT {
        self.x()
    }
    pub fn height(&self) -> YT {
        self.y()
    }

    pub fn u_mut(&mut self) -> &mut XT {
        self.x_mut()
    }
    pub fn v_mut(&mut self) -> &mut YT {
        self.y_mut()
    }
    pub fn width_mut(&mut self) -> &mut XT {
        self.x_mut()
    }
    pub fn height_mut(&mut self) -> &mut YT {
        self.y_mut()
    }

    pub fn set_u(&mut self, u: XT) -> &mut Self {
        self.set_x(u)
    }
    pub fn set_v(&mut self, v: YT) -> &mut Self {
        self.set_y(v)
    }
    pub fn set_width(&mut self, width: XT) -> &mut Self {
        self.set_x(width)
    }
    pub fn set_height(&mut self, height: YT) -> &mut Self {
        self.set_y(height)
    }
}

pub type Size<XT = i32, YT = i32> = Coordinates<XT, YT>;
pub type UVCoordinates<XT = i16, YT = i16> = Coordinates<XT, YT>;
