use crate::model::ClientId;

#[derive(Debug)]
pub(crate) struct Slots<T> {
    entries: Box<[Option<T>]>,
    len: usize,
}

impl<T> Slots<T> {
    pub(crate) fn new(capacity: u8) -> Self {
        Self { entries: (0..capacity).map(|_| None).collect(), len: 0 }
    }

    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn capacity(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn insert_with<E>(
        &mut self,
        init: impl FnOnce(ClientId) -> Result<T, E>,
    ) -> Result<Option<ClientId>, E> {
        let Some(index) = self.entries.iter().position(Option::is_none) else { return Ok(None) };
        let Ok(raw) = u8::try_from(index) else { return Ok(None) };

        let id = ClientId::new(raw);
        self.entries[index] = Some(init(id)?);
        self.len += 1;
        Ok(Some(id))
    }

    pub(crate) fn get(&self, id: ClientId) -> Option<&T> {
        self.entries.get(usize::from(id.get()))?.as_ref()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (ClientId, &T)> {
        self.entries.iter().enumerate().filter_map(|(index, entry)| {
            Some((ClientId::new(u8::try_from(index).ok()?), entry.as_ref()?))
        })
    }

    pub(crate) fn get_mut(&mut self, id: ClientId) -> Option<&mut T> {
        self.entries.get_mut(usize::from(id.get()))?.as_mut()
    }

    pub(crate) fn remove(&mut self, id: ClientId) -> Option<T> {
        let value = self.entries.get_mut(usize::from(id.get()))?.take()?;
        self.len -= 1;
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    fn insert(slots: &mut Slots<()>) -> Option<ClientId> {
        slots.insert_with(|_| Ok::<_, Infallible>(())).unwrap_or_else(|never| match never {})
    }

    #[test]
    fn hands_out_the_lowest_free_slot() {
        let mut slots = Slots::new(3);
        let a = insert(&mut slots).unwrap();
        let b = insert(&mut slots).unwrap();
        assert_eq!((a.get(), b.get()), (0, 1));

        assert!(slots.remove(a).is_some());
        assert_eq!(insert(&mut slots), Some(a));
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn refuses_when_full_and_on_unknown_ids() {
        let mut slots = Slots::new(1);
        let id = insert(&mut slots).unwrap();
        assert_eq!(insert(&mut slots), None);
        assert!(slots.remove(id).is_some());
        assert!(slots.remove(id).is_none());
        assert!(slots.remove(ClientId::new(200)).is_none());
        assert_eq!(slots.capacity(), 1);
    }
}
