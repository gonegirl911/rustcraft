use generic_array::{
    functional::FunctionalSequence,
    sequence::GenericSequence,
    typenum::{bit::B1, Add1, Unsigned},
    ArrayLength, GenericArray, GenericArrayIter,
};
use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    fmt::{self, Debug},
    iter::Zip,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Add, Index, IndexMut},
    slice,
};

pub use macros::Enum;

#[macro_export]
macro_rules! enum_map {
    ($($t:tt)*) => {
        $crate::shared::enum_map::EnumMap::from_fn(|variant| match variant {
            $($t)*
        })
    };
}

pub struct EnumMap<E: Enum, T>(GenericArray<T, E::Length>);

impl<E: Enum, T> EnumMap<E, T> {
    pub fn from_fn<F: FnMut(E) -> T>(mut f: F) -> Self {
        Self(GenericArray::generate(|i| {
            f(unsafe { E::from_index_unchecked(i) })
        }))
    }

    fn uninit() -> EnumMap<E, MaybeUninit<T>> {
        EnumMap(GenericArray::uninit())
    }

    pub fn iter(&self) -> Zip<Variants<E>, slice::Iter<T>> {
        E::variants().zip(&self.0)
    }

    pub fn values(&self) -> slice::Iter<T> {
        self.0.iter()
    }

    fn values_mut(&mut self) -> slice::IterMut<T> {
        self.0.iter_mut()
    }

    pub fn into_values(self) -> GenericArrayIter<T, E::Length> {
        self.0.into_iter()
    }

    pub fn map<U, F>(self, mut f: F) -> EnumMap<E, U>
    where
        F: FnMut(E, T) -> U,
    {
        let mut variants = E::variants();
        EnumMap(
            self.0
                .map(|value| f(unsafe { variants.next().unwrap_unchecked() }, value)),
        )
    }
}

impl<E: Enum, T> EnumMap<E, MaybeUninit<T>> {
    unsafe fn assume_init(self) -> EnumMap<E, T> {
        EnumMap(unsafe { GenericArray::assume_init(self.0) })
    }
}

impl<E: Enum, T> FromIterator<(E, T)> for EnumMap<E, T> {
    fn from_iter<I: IntoIterator<Item = (E, T)>>(iter: I) -> Self {
        let mut uninit = EnumMap::uninit();
        let mut guard = Guard::new(&mut uninit);

        for (variant, value) in iter {
            guard.set(variant, value);
        }

        assert!(guard.finish().is_ok());
        unsafe { uninit.assume_init() }
    }
}

impl<E: Enum, T: Clone> Clone for EnumMap<E, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E: Enum, T: Clone> Copy for EnumMap<E, T> where GenericArray<T, E::Length>: Copy {}

impl<E: Enum, T: PartialEq> PartialEq for EnumMap<E, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<E: Enum, T: Default> Default for EnumMap<E, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<E: Enum, T> Index<E> for EnumMap<E, T> {
    type Output = T;

    fn index(&self, variant: E) -> &Self::Output {
        unsafe { self.0.get_unchecked(variant.to_index()) }
    }
}

impl<E: Enum, T> IndexMut<E> for EnumMap<E, T> {
    fn index_mut(&mut self, variant: E) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(variant.to_index()) }
    }
}

impl<E: Enum, T> IntoIterator for EnumMap<E, T> {
    type Item = (E, T);
    type IntoIter = Zip<Variants<E>, GenericArrayIter<T, E::Length>>;

    fn into_iter(self) -> Self::IntoIter {
        E::variants().zip(self.0)
    }
}

impl<'a, E: Enum, T> IntoIterator for &'a EnumMap<E, T> {
    type Item = (E, &'a T);
    type IntoIter = Zip<Variants<E>, slice::Iter<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'de, E, T> Deserialize<'de> for EnumMap<E, T>
where
    E: Enum + Deserialize<'de> + Debug,
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapVisitor<E: Enum, T>(PhantomData<fn() -> EnumMap<E, T>>);

        impl<'de, E, T> Visitor<'de> for MapVisitor<E, T>
        where
            E: Enum + Deserialize<'de> + Debug,
            T: Deserialize<'de>,
        {
            type Value = EnumMap<E, T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a map")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
                let mut uninit = EnumMap::<E, _>::uninit();
                let mut guard = Guard::new(&mut uninit);

                while let Some((variant, value)) = access.next_entry()? {
                    if !guard.init(variant, value) {
                        return Err(M::Error::custom(format!(
                            "duplicate variant \"{variant:?}\""
                        )));
                    }
                }

                if let Err(variant) = guard.finish() {
                    Err(M::Error::custom(format!("missing variant \"{variant:?}\"")))
                } else {
                    Ok(unsafe { uninit.assume_init() })
                }
            }
        }

        deserializer.deserialize_map(MapVisitor(PhantomData))
    }
}

struct Guard<'a, E: Enum, T> {
    uninit: &'a mut EnumMap<E, MaybeUninit<T>>,
    is_init: EnumMap<E, bool>,
    count: usize,
}

impl<'a, E: Enum, T> Guard<'a, E, T> {
    fn new(uninit: &'a mut EnumMap<E, MaybeUninit<T>>) -> Self {
        Self {
            uninit,
            is_init: Default::default(),
            count: 0,
        }
    }

    fn init(&mut self, variant: E, value: T) -> bool {
        if !self.is_init[variant] {
            self.set(variant, value);
            true
        } else {
            false
        }
    }

    fn set(&mut self, variant: E, value: T) {
        self.uninit[variant].write(value);
        self.is_init[variant] = true;
        self.count += 1;
    }

    fn finish(self) -> Result<(), E> {
        if self.count == E::LEN {
            mem::forget(self);
            Ok(())
        } else {
            Err(self.missing_variant().unwrap_or_else(|| unreachable!()))
        }
    }

    fn missing_variant(&self) -> Option<E> {
        self.is_init
            .iter()
            .find(|(_, &is_init)| !is_init)
            .map(|(variant, _)| variant)
    }
}

impl<E: Enum, T> Drop for Guard<'_, E, T> {
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            for (uninit, &is_init) in self.uninit.values_mut().zip(self.is_init.values()) {
                if is_init {
                    unsafe {
                        uninit.assume_init_drop();
                    }
                }
            }
        }
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe trait Enum: Copy {
    type Length: ArrayLength;

    const LEN: usize = Self::Length::USIZE;

    fn from_index(index: usize) -> Option<Self> {
        (index < Self::LEN).then(|| unsafe { Self::from_index_unchecked(index) })
    }

    unsafe fn from_index_unchecked(index: usize) -> Self;

    fn to_index(self) -> usize;

    fn variants() -> Variants<Self> {
        Variants {
            index: 0,
            phantom: PhantomData,
        }
    }
}

unsafe impl<E: Enum> Enum for Option<E>
where
    E::Length: Add<B1>,
    Add1<E::Length>: ArrayLength,
{
    type Length = Add1<E::Length>;

    unsafe fn from_index_unchecked(index: usize) -> Self {
        E::from_index(index)
    }

    fn to_index(self) -> usize {
        self.map_or(E::LEN, Enum::to_index)
    }
}

pub struct Variants<E> {
    index: usize,
    phantom: PhantomData<fn() -> E>,
}

impl<E: Enum> Iterator for Variants<E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        let value = E::from_index(self.index)?;
        self.index += 1;
        Some(value)
    }
}
