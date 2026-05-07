#![allow(missing_docs, dead_code)]

use std::fs::File;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use fst::{Map, Set, Streamer};
use memmap2::Mmap;
use rkyv::rancor::Error as RkyvError;
use rkyv::vec::ArchivedVec;

use crate::Error;
use crate::data::{RkyvDe, RkyvSer, Saveable};

pub struct FstMap<T>
where
    T: RkyvSer,
    T::Archived: RkyvDe<T>,
{
    fst: Map<Mmap>,
    values: Mmap,
    _t: PhantomData<fn() -> T>,
}

impl<T> FstMap<T>
where
    T: RkyvSer + Saveable,
    T::Archived: RkyvDe<T>,
{
    pub fn open(dir: &Path) -> crate::Result<Self> {
        Self::open_paths(
            &dir.join(format!("{}.fst", T::base_name())),
            &dir.join(format!("{}.bin", T::base_name())),
        )
    }
}

impl<T> FstMap<T>
where
    T: RkyvSer,
    T::Archived: RkyvDe<T>,
{
    pub fn open_paths(fst_path: &Path, values_path: &Path) -> crate::Result<Self> {
        let fst_file =
            File::open(fst_path).map_err(|_| Error::MissingData(fst_path.display().to_string()))?;
        let values_file = File::open(values_path)
            .map_err(|_| Error::MissingData(values_path.display().to_string()))?;
        let fst_mmap = unsafe { Mmap::map(&fst_file)? };
        let values = unsafe { Mmap::map(&values_file)? };
        let fst = Map::new(fst_mmap).map_err(|e| Error::MissingData(e.to_string()))?;
        Ok(FstMap {
            fst,
            values,
            _t: PhantomData,
        })
    }

    pub fn get(&self, key: &str) -> Option<Vec<T>> {
        let combined = self.fst.get(key)?;
        let offset = (combined >> 32) as usize;
        let len = (combined & 0xFFFF_FFFF) as usize;
        let bytes = &self.values[offset..offset + len];
        let archived = unsafe { rkyv::access_unchecked::<ArchivedVec<T::Archived>>(bytes) };
        rkyv::deserialize::<Vec<T>, RkyvError>(archived).ok()
    }

    pub fn keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut stream = self.fst.stream();
        while let Some((k, _)) = stream.next() {
            out.push(String::from_utf8_lossy(k).into_owned());
        }
        out
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.fst.contains_key(key)
    }

    pub fn len(&self) -> u64 {
        self.fst.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.fst.is_empty()
    }
}

pub struct FstSet {
    set: Set<Mmap>,
}

impl FstSet {
    pub fn open(fst_path: &Path) -> crate::Result<Self> {
        let f =
            File::open(fst_path).map_err(|_| Error::MissingData(fst_path.display().to_string()))?;
        let mmap = unsafe { Mmap::map(&f)? };
        let set = Set::new(mmap).map_err(|e| Error::MissingData(e.to_string()))?;
        Ok(FstSet { set })
    }

    pub fn keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut stream = self.set.stream();
        while let Some(k) = stream.next() {
            out.push(String::from_utf8_lossy(k).into_owned());
        }
        out
    }

    pub fn contains(&self, key: &str) -> bool {
        self.set.contains(key)
    }

    pub fn len(&self) -> u64 {
        self.set.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("VIN_DECODE_DATA_DIR") {
        return PathBuf::from(p);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".vin-decode-cache");
    }
    PathBuf::from(".vin-decode-cache")
}
