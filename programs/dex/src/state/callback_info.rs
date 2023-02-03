use agnostic_orderbook::state::orderbook::CallbackInfo as CallbackInfoTrait;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use bytemuck::{Pod, Zeroable};

use crate::utils::loadable::Loadable;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AnchorSerialize, AnchorDeserialize, PartialEq)]
/// Buffer attached to aaob events to tie owner to events
pub struct CallBackInfoDex {
    pub user_account: Pubkey,
    pub open_orders_idx: u64,
}

impl CallBackInfoDex {
    pub fn to_vec(&self) -> Vec<u8> {
        [
            self.user_account.to_bytes().to_vec(),
            self.open_orders_idx.to_le_bytes().to_vec(),
        ]
        .concat()
    }
}

impl CallbackInfoTrait for CallBackInfoDex {
    type CallbackId = Pubkey;

    fn as_callback_id(&self) -> &Self::CallbackId {
        &self.user_account
    }
}