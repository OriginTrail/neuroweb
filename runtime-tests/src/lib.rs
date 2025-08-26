#![cfg(test)]

mod assets;

use neuroweb_runtime::{Runtime, System};
use sp_runtime::BuildStorage;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .expect("Storage should build");

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

pub fn run_to_block(n: u32) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}
