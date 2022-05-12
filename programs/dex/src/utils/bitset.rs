use bytemuck::{Pod, Zeroable};

use crate::{
    error::{DexError, DomainOrProgramResult},
    UtilError,
};
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Pod,
    Deserialize,
    Serialize,
    AnchorSerialize,
    AnchorDeserialize,
)] // serde
#[repr(C)]
// can make generic over number of u128s if necessary using Bitset<const N: usize>(pub [u128; N]);
pub struct Bitset {
    pub inner: [u128; 2],
}

unsafe impl Zeroable for Bitset {}

impl Bitset {
    #[inline]
    pub fn find_idx_and_insert(&mut self) -> DomainOrProgramResult<usize> {
        let idx = if self.inner[0] != u128::MAX {
            (u128::MAX ^ self.inner[0]).trailing_zeros()
        } else if self.inner[1] == u128::MAX {
            return Err(UtilError::InvalidBitsetIndex.into());
        } else {
            (u128::MAX ^ self.inner[1]).trailing_zeros() + 128
        } as usize;
        self.insert(idx).map(|_| idx)
    }

    #[inline]
    pub fn insert(&mut self, x: usize) -> DomainOrProgramResult {
        if x > 255 {
            return Err(UtilError::InvalidBitsetIndex.into());
        }
        self.inner[idx(x)] |= mask(x, idx(x));
        Ok(())
    }

    #[inline]
    pub fn remove(&mut self, x: usize) -> DomainOrProgramResult {
        if x > 255 {
            return Err(UtilError::InvalidBitsetIndex.into());
        }
        self.inner[idx(x)] &= !mask(x, idx(x));
        Ok(())
    }

    #[inline]
    pub fn contains(&self, x: usize) -> bool {
        if x > 255 {
            return false;
        }
        (self.inner[idx(x)] & mask(x, idx(x))) != 0
    }
}

#[inline]
fn idx(x: usize) -> usize {
    (x > 127) as usize
}

#[inline]
fn mask(x: usize, idx: usize) -> u128 {
    1 << (x - idx * 128)
}

impl Default for Bitset {
    fn default() -> Self {
        Self { inner: [0, 0] }
    }
}

mod bitset_tests {
    use crate::{error::DomainOrProgramResult, utils::bitset::Bitset};

    #[test]
    fn insert_remove_contains() -> DomainOrProgramResult {
        let mut set = Bitset::default();
        assert!(!set.contains(5));

        set.insert(2)?;
        assert!(set.contains(2));

        set.remove(2)?;
        assert!(!set.contains(2));

        set.insert(19)?;
        assert!(set.contains(19));

        set.insert(129)?;
        assert!(set.contains(129));

        set.insert(255)?;
        assert!(set.contains(255));

        assert!(set.insert(256).is_err());

        set.remove(129)?;
        assert!(!set.contains(129));
        Ok(())
    }

    #[test]
    fn find_index_and_insert() {
        let mut set = Bitset::default();
        assert_eq!(set.find_idx_and_insert().unwrap(), 0);
        assert!(set.contains(0));
        assert_eq!(set.inner, [1, 0]);
        set.remove(0).unwrap();
        assert!(!set.contains(0));
        assert_eq!(set.inner, [0, 0]);

        // fill it up
        for i in 0..256 {
            assert_eq!(set.find_idx_and_insert().unwrap(), i);
        }
        assert_eq!(set.inner, [u128::MAX; 2]);

        for i in 0..256 {
            assert!(set.contains(i));
        }
        set.remove(111).unwrap();
        assert!(!set.contains(111));
        set.remove(175).unwrap();
        assert!(!set.contains(175));
    }

    #[test]
    fn contains() {
        let set = Bitset { inner: [4, 2] };
        assert!(set.contains(2));
        assert!(!set.contains(3));
        assert!(set.contains(129));
    }
}
