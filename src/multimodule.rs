use std::array;

use crate::Key;

pub struct MultiModule<T, U, const N: usize> {
    modules: [T; N],
    on_modules: [U; N],
}

impl<T: Default, U: Default, const N: usize> Default for MultiModule<T, U, N> {
    fn default() -> Self {
        Self {
            modules: array::from_fn(|_| T::default()),
            on_modules: array::from_fn(|_| U::default()),
        }
    }
}

impl<T: Default, U: Default, const N: usize> MultiModule<T, U, N> {
    pub fn per_key(&mut self, keys: &Vec<Key>, mut f: impl FnMut(&mut T, Option<&mut U>, &Key)) {
        let mut i = 0;
        let mut on_i = 0;
        for key in keys {
            let module = &mut self.modules[i];
            if key.on {
                let on_module = &mut self.on_modules[on_i];
                f(module, Some(on_module), key);
            } else {
                f(module, None, key);
            }
            if i == on_i {
                on_i += 1;
            }
            i += 1;
        }
    }
}
