mod model;

use std::marker::PhantomData;

use smallvec::ToSmallVec;

use katana_db::abstraction::{DbCursor, DbTxMut};
use katana_db::tables;
use model::{TrieDatabaseKey, TrieDatabaseKeyType, TrieDatabaseValue};

use crate::id::BasicId;
use crate::{BonsaiDatabase, BonsaiPersistentDatabase, ByteVec, DBError, DatabaseKey};

#[derive(Debug)]
pub struct Error(katana_db::error::DatabaseError);

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl DBError for Error {}

#[derive(Debug)]
pub struct TrieDb<Tb: tables::Table, Tx: DbTxMut> {
    tx: Tx,
    _table: PhantomData<Tb>,
}

impl<Tb, Tx> TrieDb<Tb, Tx>
where
    Tb: tables::Table,
    Tx: DbTxMut,
{
    pub fn new(tx: Tx) -> Self {
        Self {
            tx,
            _table: PhantomData,
        }
    }
}

impl<Tb, Tx> BonsaiDatabase for TrieDb<Tb, Tx>
where
    Tb: tables::Table<Key = TrieDatabaseKey, Value = TrieDatabaseValue> + std::fmt::Debug,
    Tx: DbTxMut + std::fmt::Debug,
{
    type Batch = ();
    type DatabaseError = Error;

    fn create_batch(&self) -> Self::Batch {}

    fn remove_by_prefix(&mut self, prefix: &DatabaseKey<'_>) -> Result<(), Self::DatabaseError> {
        let mut cursor = self.tx.cursor_mut::<Tb>().map_err(Error)?;
        let walker = cursor.walk(None).map_err(Error)?;

        let mut keys_to_remove = Vec::new();
        // iterate over all entries in the table
        for entry in walker {
            let (key, _) = entry.map_err(Error)?;
            if key.key.starts_with(prefix.as_slice()) {
                keys_to_remove.push(key);
            }
        }

        for key in keys_to_remove {
            let _ = self.tx.delete::<Tb>(key, None).map_err(Error)?;
        }

        Ok(())
    }

    fn get(&self, key: &DatabaseKey<'_>) -> Result<Option<ByteVec>, Self::DatabaseError> {
        let value = self.tx.get::<Tb>(to_katana_table_key(key)).map_err(Error)?;
        Ok(value)
    }

    fn get_by_prefix(
        &self,
        prefix: &DatabaseKey<'_>,
    ) -> Result<Vec<(ByteVec, ByteVec)>, Self::DatabaseError> {
        let _ = prefix;
        todo!()
    }

    fn insert(
        &mut self,
        key: &DatabaseKey<'_>,
        value: &[u8],
        _batch: Option<&mut Self::Batch>,
    ) -> Result<Option<ByteVec>, Self::DatabaseError> {
        let key = to_katana_table_key(key);
        let value: ByteVec = value.to_smallvec();
        let old_value = self.tx.get::<Tb>(key.clone()).map_err(Error)?;
        self.tx.put::<Tb>(key, value).map_err(Error)?;
        Ok(old_value)
    }

    fn remove(
        &mut self,
        key: &DatabaseKey<'_>,
        _batch: Option<&mut Self::Batch>,
    ) -> Result<Option<ByteVec>, Self::DatabaseError> {
        let key = to_katana_table_key(key);
        let old_value = self.tx.get::<Tb>(key.clone()).map_err(Error)?;
        self.tx.delete::<Tb>(key, None).map_err(Error)?;
        Ok(old_value)
    }

    fn contains(&self, key: &DatabaseKey<'_>) -> Result<bool, Self::DatabaseError> {
        let key = to_katana_table_key(key);
        let value = self.tx.get::<Tb>(key).map_err(Error)?;
        Ok(value.is_some())
    }

    fn write_batch(&mut self, _batch: Self::Batch) -> Result<(), Self::DatabaseError> {
        Ok(())
    }

    #[cfg(test)]
    fn dump_database(&self) {}
}

impl<Tb, Tx> BonsaiPersistentDatabase<BasicId> for TrieDb<Tb, Tx>
where
    Tb: tables::Table<Key = TrieDatabaseKey, Value = TrieDatabaseValue> + std::fmt::Debug,
    Tx: DbTxMut + std::fmt::Debug,
{
    type DatabaseError = Error;
    type Transaction = TrieDb<Tb, Tx>;

    fn snapshot(&mut self, _: BasicId) {}

    fn merge(&mut self, _: Self::Transaction) -> Result<(), Self::DatabaseError> {
        todo!();
    }

    fn transaction(&self, _: BasicId) -> Option<Self::Transaction> {
        todo!();
    }
}

fn to_katana_table_key(key: &DatabaseKey<'_>) -> model::TrieDatabaseKey {
    match key {
        DatabaseKey::Flat(bytes) => TrieDatabaseKey {
            key: bytes.to_vec(),
            r#type: TrieDatabaseKeyType::Flat,
        },
        DatabaseKey::Trie(bytes) => TrieDatabaseKey {
            key: bytes.to_vec(),
            r#type: TrieDatabaseKeyType::Trie,
        },
        DatabaseKey::TrieLog(bytes) => TrieDatabaseKey {
            key: bytes.to_vec(),
            r#type: TrieDatabaseKeyType::TrieLog,
        },
    }
}
