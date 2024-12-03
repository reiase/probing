use std::ops::DerefMut;
use std::sync::LazyLock;
use std::sync::RwLock;

pub struct Variable<T> {
    value: RwLock<T>,
}

impl<T: Default> Default for Variable<T> {
    fn default() -> Self {
        Self {
            value: RwLock::new(T::default()),
        }
    }
}

use std::ops::Deref;

impl<T: Default> Deref for Variable<T> {
    type Target = RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Default> DerefMut for Variable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Default> Variable<T> {
    pub fn assign(&mut self, value: T) {
        let mut store = self.value.write().unwrap();
        *store = value;
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        let store = self.value.read().unwrap();
        store.clone()
    }
}
