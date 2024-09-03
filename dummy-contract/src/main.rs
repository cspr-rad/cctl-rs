#![no_std]
#![no_main]

use casper_contract::contract_api::storage;
use casper_types::{
    addressable_entity::NamedKeys, CLType, EntryPoint, EntryPointAccess, EntryPointPayment,
    EntryPointType, EntryPoints,
};
extern crate alloc;
use alloc::{string::ToString, vec};

#[no_mangle]
pub extern "C" fn dummy() {
    // this doesn't do anything, but you can call it.
}

#[no_mangle]
pub extern "C" fn call() {
    let entry_points = {
        let dummy = EntryPoint::new(
            "dummy",
            vec![],
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
            EntryPointPayment::Caller,
        );

        let mut entry_points = EntryPoints::new();
        entry_points.add_entry_point(dummy);
        entry_points
    };
    let named_keys = NamedKeys::new();
    storage::new_contract(
        entry_points,
        Some(named_keys),
        Some("contract-hash".to_string()),
        Some("contract-package-hash".to_string()),
        None,
    );
}
