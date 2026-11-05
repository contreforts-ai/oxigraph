//! Adapter that plugs the pure-Rust [`oxkv`] engine in place of
//! `rocksdb_wrapper.rs` (selected by the `oxkv` feature).
//!
//! `oxkv` exposes the same API as `rocksdb_wrapper.rs` but cannot name
//! Oxigraph's error types. The types returning a `Result` are wrapped here to
//! convert [`oxkv::KvError`] into [`StorageError`], so that `rocksdb.rs` works
//! unchanged with either backend.

use crate::storage::error::{CorruptionError, StorageError};
pub use oxkv::{ColumnFamily, ColumnFamilyDefinition, DbOptions, PinnableSlice};
use std::path::{Path, PathBuf};

impl From<oxkv::KvError> for StorageError {
    #[inline]
    fn from(error: oxkv::KvError) -> Self {
        match error {
            oxkv::KvError::Io(e) => Self::Io(e),
            oxkv::KvError::Corruption(e) => Self::Corruption(CorruptionError::new(e)),
            oxkv::KvError::MissingColumnFamily(name) => {
                Self::Corruption(CorruptionError::from_missing_column_family_name(name))
            }
            oxkv::KvError::Other(e) => Self::Other(e),
            e => Self::Other(Box::new(e)),
        }
    }
}

#[derive(Clone)]
pub struct Db(oxkv::Db);

impl Db {
    pub fn open_read_write(
        path: &Path,
        column_families: Vec<ColumnFamilyDefinition>,
        db_options: DbOptions,
    ) -> Result<Self, StorageError> {
        Ok(Self(oxkv::Db::open_read_write(
            path,
            column_families,
            db_options,
        )?))
    }

    pub fn open_read_only(
        path: &Path,
        column_families: Vec<ColumnFamilyDefinition>,
    ) -> Result<Self, StorageError> {
        Ok(Self(oxkv::Db::open_read_only(path, column_families)?))
    }

    pub fn is_writable(&self) -> bool {
        self.0.is_writable()
    }

    pub fn column_family(&self, name: &'static str) -> Result<ColumnFamily, StorageError> {
        Ok(self.0.column_family(name)?)
    }

    #[must_use]
    pub fn snapshot(&self) -> Reader<'static> {
        Reader(self.0.snapshot())
    }

    pub fn start_transaction(&self) -> Result<Transaction, StorageError> {
        Ok(Transaction(self.0.start_transaction()?))
    }

    pub fn start_readable_transaction(&self) -> Result<ReadableTransaction<'_>, StorageError> {
        Ok(ReadableTransaction(self.0.start_readable_transaction()?))
    }

    pub fn get(
        &self,
        column_family: &ColumnFamily,
        key: &[u8],
    ) -> Result<Option<PinnableSlice>, StorageError> {
        Ok(self.0.get(column_family, key)?)
    }

    pub fn contains_key(
        &self,
        column_family: &ColumnFamily,
        key: &[u8],
    ) -> Result<bool, StorageError> {
        Ok(self.0.contains_key(column_family, key)?)
    }

    pub fn insert(
        &self,
        column_family: &ColumnFamily,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), StorageError> {
        Ok(self.0.insert(column_family, key, value)?)
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        Ok(self.0.flush()?)
    }

    pub fn compact(&self, column_family: &ColumnFamily) -> Result<(), StorageError> {
        Ok(self.0.compact(column_family)?)
    }

    pub fn new_sst_file(&self) -> Result<SstFileWriter, StorageError> {
        Ok(SstFileWriter(self.0.new_sst_file()?))
    }

    pub fn insert_stt_files(
        &self,
        ssts_for_cf: &[(ColumnFamily, PathBuf)],
    ) -> Result<(), StorageError> {
        Ok(self.0.insert_stt_files(ssts_for_cf)?)
    }

    pub fn backup(&self, target_directory: &Path) -> Result<(), StorageError> {
        Ok(self.0.backup(target_directory)?)
    }
}

#[derive(Clone)]
pub struct Reader<'a>(oxkv::Reader<'a>);

impl<'a> Reader<'a> {
    pub fn get(
        &self,
        column_family: &ColumnFamily,
        key: &[u8],
    ) -> Result<Option<PinnableSlice>, StorageError> {
        Ok(self.0.get(column_family, key)?)
    }

    pub fn contains_key(
        &self,
        column_family: &ColumnFamily,
        key: &[u8],
    ) -> Result<bool, StorageError> {
        Ok(self.0.contains_key(column_family, key)?)
    }

    #[expect(clippy::iter_not_returning_iterator)]
    pub fn iter(&self, column_family: &ColumnFamily) -> Iter<'a> {
        Iter(self.0.iter(column_family))
    }

    pub fn scan_prefix(&self, column_family: &ColumnFamily, prefix: &[u8]) -> Iter<'a> {
        Iter(self.0.scan_prefix(column_family, prefix))
    }

    pub fn len(&self, column_family: &ColumnFamily) -> Result<usize, StorageError> {
        Ok(self.0.len(column_family)?)
    }

    pub fn is_empty(&self, column_family: &ColumnFamily) -> Result<bool, StorageError> {
        Ok(self.0.is_empty(column_family)?)
    }
}

pub struct Transaction(oxkv::Transaction);

impl Transaction {
    pub fn insert(&mut self, column_family: &ColumnFamily, key: &[u8], value: &[u8]) {
        self.0.insert(column_family, key, value)
    }

    pub fn insert_empty(&mut self, column_family: &ColumnFamily, key: &[u8]) {
        self.0.insert_empty(column_family, key)
    }

    pub fn remove(&mut self, column_family: &ColumnFamily, key: &[u8]) {
        self.0.remove(column_family, key)
    }

    pub fn remove_range(&mut self, column_family: &ColumnFamily, start_key: &[u8], end_key: &[u8]) {
        self.0.remove_range(column_family, start_key, end_key)
    }

    pub fn commit(self) -> Result<(), StorageError> {
        Ok(self.0.commit()?)
    }
}

pub struct ReadableTransaction<'a>(oxkv::ReadableTransaction<'a>);

impl ReadableTransaction<'_> {
    pub fn reader(&self) -> Reader<'_> {
        Reader(self.0.reader())
    }

    pub fn insert(&mut self, column_family: &ColumnFamily, key: &[u8], value: &[u8]) {
        self.0.insert(column_family, key, value)
    }

    pub fn insert_empty(&mut self, column_family: &ColumnFamily, key: &[u8]) {
        self.0.insert_empty(column_family, key)
    }

    pub fn remove(&mut self, column_family: &ColumnFamily, key: &[u8]) {
        self.0.remove(column_family, key)
    }

    pub fn commit(self) -> Result<(), StorageError> {
        Ok(self.0.commit()?)
    }
}

pub struct Iter<'a>(oxkv::Iter<'a>);

impl Iter<'_> {
    pub fn status(&self) -> Result<(), StorageError> {
        Ok(self.0.status()?)
    }

    pub fn next(&mut self) {
        self.0.next()
    }

    pub fn key(&self) -> Option<&[u8]> {
        self.0.key()
    }
}

pub struct SstFileWriter(oxkv::SstFileWriter);

impl SstFileWriter {
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        Ok(self.0.insert(key, value)?)
    }

    pub fn insert_empty(&mut self, key: &[u8]) -> Result<(), StorageError> {
        Ok(self.0.insert_empty(key)?)
    }

    pub fn finish(self) -> Result<PathBuf, StorageError> {
        Ok(self.0.finish()?)
    }
}
