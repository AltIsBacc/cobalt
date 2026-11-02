use cobalt_protocol::BuffList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Item {
    pub stack: i16,
    pub prefix: u8,
    pub net_id: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub name: String,
    pub uuid: String,
    pub hp: i16,
    pub max_hp: i16,
    pub mana: i16,
    pub max_mana: i16,
    pub buffs: BuffList,
    pub inventory: Box<[Item]>,
}

impl Player {
    pub const MAX_NAME_LEN: usize = 20;
    pub const INVENTORY_SLOTS: usize = 990;

    #[must_use]
    pub fn has_valid_name(&self) -> bool {
        let len = self.name.chars().count();
        (1..=Self::MAX_NAME_LEN).contains(&len) && !self.name.trim().is_empty()
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: String::new(),
            uuid: String::new(),
            hp: 0,
            max_hp: 0,
            mana: 0,
            max_mana: 0,
            buffs: BuffList::default(),
            inventory: vec![Item::default(); Self::INVENTORY_SLOTS].into_boxed_slice(),
        }
    }
}
