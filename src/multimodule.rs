use std::array;

use crate::Key;

pub struct MultiModule<T, const N: usize> {
    modules: [T; N],
}

impl<T: Default, const N: usize> Default for MultiModule<T, N> {
    fn default() -> Self {
        Self {
            modules: array::from_fn(|_| T::default()),
        }
    }
}

impl<T: Default, const N: usize> MultiModule<T, N> {
    pub fn per_key(&mut self, keys: Vec<Key>, mut f: impl FnMut(&mut T, Key)) {
        for (module, key) in self.modules.iter_mut().zip(keys) {
            f(module, key);
        }
    }
    pub fn per_key_on(&mut self, keys: Vec<Key>, mut f: impl FnMut(&mut T, Key)) {
        for (module, key) in self.modules.iter_mut().zip(keys).filter(|(_, k)| k.on) {
            f(module, key);
        }
    }
}
