use std::fmt;

use crate::player::Player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u8);

impl ClientId {
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug)]
pub(crate) struct Clients {
    used: Box<[bool]>,
    players: Box<[Option<Player>]>,
    count: usize,
}

impl Clients {
    pub(crate) fn new(capacity: u8) -> Self {
        Self {
            used: vec![false; usize::from(capacity)].into_boxed_slice(),
            players: (0..capacity).map(|_| None).collect(),
            count: 0,
        }
    }

    pub(crate) const fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn capacity(&self) -> usize {
        self.used.len()
    }

    pub(crate) fn insert(&mut self) -> Option<ClientId> {
        let index = self.used.iter().position(|used| !used)?;
        let id = ClientId(u8::try_from(index).ok()?);
        self.used[index] = true;
        self.count += 1;
        Some(id)
    }

    pub(crate) fn set_player(&mut self, id: ClientId, player: Player) -> bool {
        let index = usize::from(id.0);
        match self.players.get_mut(index) {
            Some(slot) if self.used[index] => {
                *slot = Some(player);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn remove(&mut self, id: ClientId) -> bool {
        let index = usize::from(id.0);
        match self.used.get_mut(index) {
            Some(slot) if *slot => {
                *slot = false;
                self.players[index] = None;
                self.count -= 1;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_out_the_lowest_free_slot() {
        let mut clients = Clients::new(3);
        let a = clients.insert().unwrap();
        let b = clients.insert().unwrap();
        assert_eq!((a.get(), b.get()), (0, 1));

        assert!(clients.remove(a));
        assert_eq!(clients.insert(), Some(a));
        assert_eq!(clients.count(), 2);
    }

    #[test]
    fn refuses_when_full_and_on_unknown_ids() {
        let mut clients = Clients::new(1);
        let id = clients.insert().unwrap();
        assert_eq!(clients.insert(), None);
        assert!(clients.remove(id));
        assert!(!clients.remove(id));
        assert!(!clients.remove(ClientId(200)));
        assert_eq!(clients.capacity(), 1);
    }
}
