use std::time::Duration;

use cobalt_format::world::World;

use crate::traits::tickable::Tickable;


impl Tickable for World {
    fn tick(&mut self, _dt: f32) {
        // println!("ticked! {:.2}", dt);
    }
}

