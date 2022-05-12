use std::{
    any::type_name,
    cell::{Ref, RefMut},
    mem,
    mem::size_of,
};

use anchor_lang::solana_program::{account_info::AccountInfo, msg};
use bytemuck::{Pod, PodCastError};

use crate::error::{DexError, DomainOrProgramError};

fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> DomainOrProgramError {
    move |_: PodCastError| -> DomainOrProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        DomainOrProgramError::DexErr(DexError::InvalidBytesForZeroCopyDeserialization)
    }
}

pub trait Loadable: Pod {
    fn load<'a>(
        account: &'a AccountInfo,
    ) -> std::result::Result<Ref<'a, Self>, DomainOrProgramError> {
        let size = mem::size_of::<Self>();
        Ok(Ref::map(account.try_borrow_data()?, |data| {
            bytemuck::try_from_bytes(&data[..size])
                .map_err(error_msg::<Self>(data.len()))
                .unwrap()
        }))
    }

    fn load_mut<'a>(
        account: &'a AccountInfo,
    ) -> std::result::Result<RefMut<'a, Self>, DomainOrProgramError> {
        let size = mem::size_of::<Self>();
        Ok(RefMut::map(account.try_borrow_mut_data()?, |data| {
            let data_len = data.len();
            bytemuck::try_from_bytes_mut(&mut data[..size])
                .map_err(error_msg::<Self>(data_len))
                .unwrap()
        }))
    }

    fn load_from_bytes(data: &[u8]) -> std::result::Result<&Self, DomainOrProgramError> {
        bytemuck::try_from_bytes(data).map_err(error_msg::<Self>(data.len()))
    }

    fn load_from_bytes_mut(
        data: &mut [u8],
    ) -> std::result::Result<&mut Self, DomainOrProgramError> {
        let data_len = data.len();
        bytemuck::try_from_bytes_mut(data).map_err(error_msg::<Self>(data_len))
    }

    #[deprecated]
    fn load_partial_mut<'a>(
        account: &'a AccountInfo,
    ) -> std::result::Result<RefMut<'a, Self>, DomainOrProgramError> {
        Loadable::load_mut(account)
    }
}

impl<T: Pod> Loadable for T {}
