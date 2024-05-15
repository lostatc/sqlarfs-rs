use std::collections::HashSet;
use std::marker::PhantomData;

// A table for allocating integer IDs.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct IdTable<Id> {
    // The highest used ID value (the high water mark).
    highest: u64,

    // A set of unused ID values below the high water mark.
    unused: HashSet<u64>,

    // The set of ID values which cannot ever be allocated.
    reserved: HashSet<u64>,

    phantom: PhantomData<Id>,
}

impl<Id> IdTable<Id>
where
    Id: From<u64> + Into<u64> + Copy,
{
    /// Return a new empty `IdTable` with the given `reserved` IDs.
    pub fn new(reserved: impl IntoIterator<Item = Id>) -> Self {
        Self {
            highest: 0,
            unused: HashSet::new(),
            reserved: reserved.into_iter().map(Into::into).collect::<HashSet<_>>(),
            phantom: PhantomData,
        }
    }

    // Return the next unused ID from the table.
    pub fn next(&mut self) -> Id {
        match self.unused.iter().next().copied() {
            Some(id) => {
                self.unused.remove(&id);
                Id::from(id)
            }
            None => {
                self.highest += 1;
                while self.reserved.contains(&self.highest) {
                    self.highest += 1;
                }
                Id::from(self.highest)
            }
        }
    }

    // Return whether the given `id` is in the table.
    pub fn contains(&self, id: Id) -> bool {
        id.into() <= self.highest && !self.unused.contains(&id.into())
    }

    // Return the given `id` back to the table.
    //
    // This returns `true` if the value was returned or `false` if it was unused or reserved.
    pub fn recycle(&mut self, id: Id) -> bool {
        if !self.contains(id) || self.reserved.contains(&id.into()) {
            return false;
        }
        self.unused.insert(id.into());
        true
    }
}
