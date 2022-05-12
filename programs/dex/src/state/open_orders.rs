use crate::error::{DexError, DomainOrProgramResult};
use anchor_lang::{
    prelude::*,
    solana_program::{msg, program_error::ProgramError},
};

use crate::utils::{
    numeric::{Fractional, ZERO_FRAC},
    validation::assert,
};

use crate::state::constants::*;

#[zero_copy]
pub struct OpenOrdersMetadata {
    pub ask_qty_in_book: Fractional,
    pub bid_qty_in_book: Fractional,
    pub head_index: usize,
    pub num_open_orders: u64,
}

#[zero_copy]
pub struct OpenOrders {
    pub free_list_head: usize,
    pub total_open_orders: u64,
    pub products: [OpenOrdersMetadata; MAX_PRODUCTS],
    pub orders: [OpenOrdersNode; MAX_OPEN_ORDERS],
}

impl OpenOrders {
    pub fn initialize(&mut self) {
        self.free_list_head = 1;
        for product_meta in self.products.iter_mut() {
            product_meta.ask_qty_in_book = ZERO_FRAC;
            product_meta.bid_qty_in_book = ZERO_FRAC;
            product_meta.head_index = 0;
            product_meta.num_open_orders = 0;
        }
    }

    pub fn has_open_order(&self, index: usize, order_id: u128) -> bool {
        let mut i = self.products[index].head_index;
        while i != SENTINEL {
            let head = self.orders[i];
            if head.id == order_id {
                return true;
            }
            i = head.next;
        }
        false
    }

    #[inline(always)]
    pub fn get_next_index(&self) -> usize {
        self.free_list_head
    }

    pub fn add_open_order(&mut self, index: usize, order_id: u128) -> DomainOrProgramResult {
        let head_index = &mut self.products[index].head_index;
        let i = *head_index as usize;
        // Fetch the index of the free node to write to
        let free_list_head = self.free_list_head;
        let free_node = &mut self.orders[free_list_head];
        let next_free_node = free_node.next;
        // Add the order id to free node
        free_node.id = order_id;
        free_node.next = i;
        free_node.prev = SENTINEL;
        // Assign this node as the new head for the index
        *head_index = free_list_head as usize;
        if i != SENTINEL {
            // If there are existing open orders for this index, we set the current head
            // to point to the updated head
            self.orders[i].prev = free_list_head;
        }
        if next_free_node == SENTINEL {
            // If there are no more free nodes, this means that the linked list is densely packed.
            // The next free node will just be the next index.
            assert(
                free_list_head + 1 < MAX_OPEN_ORDERS,
                DexError::TooManyOpenOrdersError,
            )?;
            self.free_list_head = free_list_head + 1;
        } else {
            // If there are free nodes remaining, we keep traversing the linked list.
            self.free_list_head = next_free_node;
        }
        Ok(())
    }

    fn remove_node(&mut self, index: usize, i: usize) {
        let head_index = &mut self.products[index].head_index;
        let free_list_head = self.free_list_head;
        let node = &mut self.orders[i];
        let next = node.next;
        let prev = node.prev;
        if prev == SENTINEL {
            // If we enter this block, we need to update the head of the index as we are deleting the current head.
            *head_index = next;
        }
        // In the process of deleting the current node, we add it to the head of the free list.
        node.id = 0;
        node.next = free_list_head;
        node.prev = SENTINEL;
        self.orders[free_list_head].prev = i;
        self.free_list_head = i;
        // If the node is not the head or tail, we need to modify the pointers of the prev and next nodes.
        if next != SENTINEL {
            self.orders[next].prev = prev;
        }
        if prev != SENTINEL {
            self.orders[prev].next = next;
        }
    }

    pub fn remove_open_order_by_index(
        &mut self,
        index: usize,
        i: usize,
        order_id: u128,
    ) -> DomainOrProgramResult {
        assert(
            i < MAX_OPEN_ORDERS && self.orders[i].id == order_id,
            DexError::InvalidOrderID,
        )?;
        self.remove_node(index, i);
        Ok(())
    }

    pub fn remove_open_order(&mut self, index: usize, order_id: u128) -> DomainOrProgramResult {
        let head_index = &mut self.products[index].head_index;
        let mut i = *head_index as usize;
        while i != SENTINEL {
            let node = &mut self.orders[i];
            if node.id == order_id {
                self.remove_node(index, i);
                return Ok(());
            }
            i = node.next;
        }
        Err(ProgramError::InvalidAccountData.into())
    }

    pub fn clear(&mut self, index: usize) -> DomainOrProgramResult {
        let head_index = &mut self.products[index].head_index;
        let mut i = *head_index as usize;
        while i != SENTINEL {
            let node = &mut self.orders[i];
            let next = node.next;
            self.remove_node(index, i);
            i = next;
        }
        Ok(())
    }
}

#[zero_copy]
pub struct OpenOrdersNode {
    pub id: u128,
    pub client_id: u128,
    pub prev: usize,
    pub next: usize,
}
