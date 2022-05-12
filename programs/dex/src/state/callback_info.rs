use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use bytemuck::{Pod, Zeroable};

use crate::utils::loadable::Loadable;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AnchorSerialize, AnchorDeserialize)]
/// Buffer attached to aaob events to tie owner to events
pub struct CallBackInfo {
    pub user_account: Pubkey,
    pub open_orders_idx: u64,
}

impl CallBackInfo {
    pub fn to_vec(&self) -> Vec<u8> {
        [
            self.user_account.to_bytes().to_vec(),
            self.open_orders_idx.to_le_bytes().to_vec(),
        ]
        .concat()
    }
}
