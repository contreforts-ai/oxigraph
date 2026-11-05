# oxkv

Pure-Rust storage backend for Oxigraph, selected with the `oxkv` feature of the
`oxigraph` crate:

```sh
cargo build -p oxigraph --no-default-features --features oxkv
```

**Status: placeholder.** This crate only defines the API that Oxigraph's on-disk
storage (`lib/oxigraph/src/storage/rocksdb.rs`) uses. `Db::open_read_write` and
`Db::open_read_only` return `KvError::Unimplemented`, so every other method is
unreachable. Replace the bodies with a real engine.

## Contract

The API mirrors `lib/oxigraph/src/storage/rocksdb_wrapper.rs`, the safe wrapper
around RocksDB. Oxigraph's indexes and key encoding stay in Oxigraph; this crate
is only an ordered key-value engine with:

- a fixed set of named column families, given at open time;
- point `get` / `contains_key`;
- point-in-time snapshots (`Db::snapshot`) that are `Clone + Send + Sync` and
  outlive the borrow of `Db` (`Reader<'static>`);
- ordered iteration over keys, whole column family or by prefix;
- atomic write batches with `insert`, `remove`, `remove_range` (`Transaction`);
- read-your-writes transactions: a snapshot plus pending writes, visible to
  `get` and to iteration through `ReadableTransaction::reader`;
- bulk loading: sorted files written outside the database
  (`Db::new_sst_file`), then added to several column families atomically
  (`Db::insert_stt_files`);
- `flush`, `compact` (may be no-ops), `backup` into a new directory, and
  read-only opening.

Errors map onto Oxigraph's `StorageError` in
`lib/oxigraph/src/storage/oxkv_wrapper.rs`: `KvError::Io` → `StorageError::Io`,
`KvError::Corruption` and `KvError::MissingColumnFamily` → `StorageError::Corruption`, everything else →
`StorageError::Other`.

Data written by this backend is not readable by the RocksDB backend and vice
versa; migrate with `oxigraph dump` / `oxigraph load`.

## How it plugs into Oxigraph (fork maintenance notes)

Features of the `oxigraph` crate:

| Feature       | Meaning                                                       |
|---------------|---------------------------------------------------------------|
| `rocksdb`     | On-disk storage (`Store::open` …). Needs one backend below.   |
| `rocksdb-cpp` | RocksDB backend (C++, `oxrocksdb-sys`). In `default`.          |
| `oxkv`        | This backend. Wins over `rocksdb-cpp` when both are enabled.   |

`rocksdb` keeps upstream's meaning for every `cfg(feature = "rocksdb")`, so
none of them had to change. The whole switch is in
`lib/oxigraph/src/storage/mod.rs`: with `oxkv`, the module `rocksdb_wrapper` is
loaded from `oxkv_wrapper.rs` (re-exports this crate and converts `KvError` into
`StorageError`) instead of `rocksdb_wrapper.rs` (RocksDB FFI).

Files touched outside this crate: `Cargo.toml` (workspace member and
dependency), `lib/oxigraph/Cargo.toml` (features), `storage/mod.rs` (switch),
`storage/oxkv_wrapper.rs` (new), `tests/store.rs` (the RocksDB-format fixture
test is ignored under `oxkv`), `fuzz/Cargo.toml` and the CI clippy job.

When rebasing on upstream, the only file to watch is
`lib/oxigraph/src/storage/rocksdb_wrapper.rs`: any method added or changed there
must be mirrored here and in `oxkv_wrapper.rs`. CI runs
`cargo clippy --no-default-features --features oxkv` so a mismatch fails the
build.

Acceptance tests: `cargo test -p oxigraph --no-default-features --features oxkv`.
With the placeholder, the 12 on-disk tests in `tests/store.rs` and
`storage::rocksdb::tests::test_transaction` fail with `Unimplemented`; all
in-memory tests pass.

The CLI and server still always build RocksDB (they depend on `oxigraph` with
default features). Giving them an `oxkv` option is left for when the engine
works.
