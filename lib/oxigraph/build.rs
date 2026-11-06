//! Sets the `oxigraph_on_disk` cfg when an on-disk storage backend is enabled:
//! the "rocksdb" feature (RocksDB) or the "oxnative" feature (pure Rust).
//!
//! On-disk storage is gated on this cfg rather than on the "rocksdb" feature,
//! so that a build with only "oxnative" does not enable anything named RocksDB.

use std::env;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(oxigraph_on_disk)");
    if env::var_os("CARGO_FEATURE_ROCKSDB").is_some()
        || env::var_os("CARGO_FEATURE_OXNATIVE").is_some()
    {
        println!("cargo::rustc-cfg=oxigraph_on_disk");
    }
}
