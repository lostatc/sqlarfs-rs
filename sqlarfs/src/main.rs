//! Just for testing

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use sqlarfs::Connection;

#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> sqlarfs::Result<()> {
    Connection::open("/home/wren/Desktop/test-crate.sqlar")?.exec(|archive| {
        archive
            .open("symlink")?
            .create_symlink("/home/wren/Desktop/link-target")
    })
}
