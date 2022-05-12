use crate::{error::DomainOrProgramError, utils::loadable::Loadable};
use anchor_lang::solana_program::{account_info::AccountInfo, pubkey::Pubkey};
use bytemuck::{Pod, Zeroable};
use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AcctWithDisc<T: 'static + Copy> {
    pub discriminant: u64,
    pub inner: T,
}

unsafe impl<T: 'static + Copy> Zeroable for AcctWithDisc<T> {}

unsafe impl<T: 'static + Copy> Pod for AcctWithDisc<T> {}

impl<T: 'static + Copy> DerefMut for AcctWithDisc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: 'static + Copy> Deref for AcctWithDisc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct WithAcct<'a, 'b, T> {
    pub acct: &'a AccountInfo<'b>,
    inner: T,
}

impl<'a, 'b, T> WithAcct<'a, 'b, T> {
    pub(crate) fn new(acct: &'a AccountInfo<'b>, inner: T) -> Self {
        WithAcct { acct, inner }
    }

    pub fn load_mut(
        acct: &'a AccountInfo<'b>,
    ) -> std::result::Result<WithAcct<'a, 'b, RefMut<'a, T>>, DomainOrProgramError>
    where
        T: Loadable,
    {
        Ok(WithAcct {
            acct,
            inner: T::load_mut(acct)?,
        })
    }

    pub fn load(
        acct: &'a AccountInfo<'b>,
    ) -> std::result::Result<WithAcct<'a, 'b, Ref<'a, T>>, DomainOrProgramError>
    where
        T: Loadable,
    {
        Ok(WithAcct {
            acct,
            inner: T::load(acct)?,
        })
    }
}

impl<'a, 'b, T> DerefMut for WithAcct<'a, 'b, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, 'b, T> Deref for WithAcct<'a, 'b, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct WithKey<'a, T> {
    pub key: &'a Pubkey,
    inner: T,
}

impl<'a, T> WithKey<'a, T> {
    pub fn new(key: &'a Pubkey, inner: T) -> Self {
        WithKey { key, inner }
    }

    pub fn load_mut<'b: 'a>(
        acct: &'a AccountInfo<'b>,
    ) -> std::result::Result<WithKey<'a, RefMut<'a, T>>, DomainOrProgramError>
    where
        T: Loadable,
    {
        Ok(WithKey {
            key: acct.key,
            inner: T::load_mut(acct)?,
        })
    }

    pub fn load<'b: 'a>(
        acct: &'a AccountInfo<'b>,
    ) -> std::result::Result<WithKey<'a, Ref<'a, T>>, DomainOrProgramError>
    where
        T: Loadable,
    {
        Ok(WithKey {
            key: acct.key,
            inner: T::load(acct)?,
        })
    }

    pub fn from_acct<'b: 'a>(acct: &'a AccountInfo<'b>, inner: T) -> Self {
        WithKey {
            key: acct.key,
            inner,
        }
    }
}

impl<'a, T> DerefMut for WithKey<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Deref for WithKey<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
