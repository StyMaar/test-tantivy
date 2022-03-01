use core::fmt;
use std::{path::{Path, PathBuf}, ops::Range, collections::HashMap, sync::{Arc, Mutex}, io::{BufWriter, Write}, marker::PhantomData};

use tantivy::{Directory, directory::{error::{DeleteError, OpenReadError, OpenWriteError}, FileHandle, WritePtr, WatchCallback, WatchHandle, OwnedBytes, TerminatingWrite, AntiCallToken}, TantivyError, HasLen};

use serde::{Serialize,Deserialize, Serializer, Deserializer, de::Visitor};

#[derive(Debug, Clone)]
pub struct HashMapDirectory(Arc<Mutex<HashMap<PathBuf, HashMapFile>>>);

impl HashMapDirectory {
    pub fn new()-> Self {
        HashMapDirectory(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl Directory for HashMapDirectory {
    fn get_file_handle(
        &self, 
        path: &Path
    ) -> Result<Box<dyn FileHandle>, OpenReadError>{

        match self.0.lock().unwrap().get(path) {
            None => Err(OpenReadError::FileDoesNotExist(path.into())),
            Some(buffer_pointer) => {
                Ok(Box::new(buffer_pointer.clone()))
            }
        }
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError>{
        match self.0.lock().unwrap().remove(path) {
            None => Err(DeleteError::FileDoesNotExist(path.into())),
            Some(_) => {
                Ok(())
            }
        }
    }
    
    fn exists(&self, path: &Path) -> Result<bool, OpenReadError>{
        Ok(self.0.lock().unwrap().contains_key(path))
    }
    
    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError>{
        let mut hash_map_directory = self.0.lock().unwrap();
        let buffer_pointer = hash_map_directory.entry(path.to_path_buf()).or_insert(HashMapFile(Arc::new(Mutex::new(Vec::new()))));        
        Ok(BufWriter::new(Box::new(buffer_pointer.clone())))
    }
    
    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError>{
        match self.0.lock().unwrap().get(path) {
            None => Err(OpenReadError::FileDoesNotExist(path.into())),
            Some(buffer_pointer) => {
                Ok(buffer_pointer.0.lock().unwrap().clone())
            }
        }
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> std::io::Result<()>{
        let buffer_pointer = {
            let mut hash_map_directory = self.0.lock().unwrap();
            hash_map_directory.entry(path.to_path_buf()).or_insert(HashMapFile(Arc::new(Mutex::new(Vec::new())))).clone()
        };
        let mut buffer_data = buffer_pointer.0.lock().unwrap();
        buffer_data.clear();
        buffer_data.extend_from_slice(data);
        Ok(())
    }
    
    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle>{
        Ok(WatchHandle::empty())
    }

}

#[derive(Debug, Clone)]
struct HashMapFile(Arc<Mutex<Vec<u8>>>);

impl TerminatingWrite for HashMapFile{
    fn terminate_ref(&mut self, _: AntiCallToken) -> std::io::Result<()>{
        self.flush()
    }
} 

impl Write for HashMapFile{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>{
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()>{
        Ok(())
    }
}

impl HasLen for HashMapFile {
    fn len(&self) -> usize{
        self.0.lock().unwrap().len()
    }
}
impl FileHandle for HashMapFile {
    fn read_bytes(&self, range: Range<usize>) -> std::io::Result<OwnedBytes>{

        let bytes = self.0.lock()
            .unwrap()
            .get(range.clone())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("Trying to fetch data out of range: {range:?}")))?
            .to_owned();

        Ok(OwnedBytes::new(bytes))
    }
}

impl Serialize for HashMapDirectory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("HashMapDirectory", &*self.0.lock().unwrap())
    }
}

struct HashMapDirectoryVisitor {
    marker: PhantomData<fn() -> HashMapDirectory>
}

impl HashMapDirectoryVisitor {
    fn new() -> Self {
        HashMapDirectoryVisitor {
            marker: PhantomData
        }
    }
}

impl<'de> Visitor<'de> for HashMapDirectoryVisitor
{
    // The type that our Visitor is going to produce.
    type Value = HashMapDirectory;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a very special map")
    }

    fn visit_newtype_struct<D>(
        self,
        deserializer: D
    ) -> Result<Self::Value, D::Error> where
    D: Deserializer<'de> {
        Ok(HashMapDirectory(Arc::new(Mutex::new(Deserialize::deserialize(deserializer)?))))
    }

    
}

impl<'de> Deserialize<'de> for HashMapDirectory
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of MyMap.
        deserializer.deserialize_newtype_struct("HashMapDirectory", HashMapDirectoryVisitor::new())
    }
}



impl Serialize for HashMapFile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("HashMapFile", &*self.0.lock().unwrap())
    }
}


struct HashMapFileVisitor {
    marker: PhantomData<fn() -> HashMapFile>
}

impl HashMapFileVisitor {
    fn new() -> Self {
        HashMapFileVisitor {
            marker: PhantomData
        }
    }
}

impl<'de> Visitor<'de> for HashMapFileVisitor
{
    // The type that our Visitor is going to produce.
    type Value = HashMapFile;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a very special map")
    }

    fn visit_newtype_struct<D>(
        self,
        deserializer: D
    ) -> Result<Self::Value, D::Error> where
    D: Deserializer<'de> {
        Ok(HashMapFile(Arc::new(Mutex::new(Deserialize::deserialize(deserializer)?))))
    }

    
}

impl<'de> Deserialize<'de> for HashMapFile
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data, resulting in an instance of MyMap.
        deserializer.deserialize_newtype_struct("HashMapDirectory", HashMapFileVisitor::new())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;

    use super::HashMapDirectory;
    use super::Directory;

    #[test]
    fn test_persist() {
        let msg_atomic: &'static [u8] = b"atomic is the way";
        let msg_seq: &'static [u8] = b"sequential is the way";
        let path_atomic: &'static Path = Path::new("atomic");
        let path_seq: &'static Path = Path::new("seq");
        let directory = HashMapDirectory::new();
        assert!(directory.atomic_write(path_atomic, msg_atomic).is_ok());
        let mut wrt = directory.open_write(path_seq).unwrap();
        assert!(wrt.write_all(msg_seq).is_ok());
        assert!(wrt.flush().is_ok());
        assert_eq!(directory.atomic_read(path_atomic).unwrap(), msg_atomic);
        assert_eq!(directory.atomic_read(path_seq).unwrap(), msg_seq);

        let msg_atomic_2: &'static [u8] = b", maybe";
        let msg_seq_2: &'static [u8] = b", maybe";

        assert!(directory.atomic_write(path_atomic, msg_atomic_2).is_ok());
        let mut wrt = directory.open_write(path_seq).unwrap();
        assert!(wrt.write_all(msg_seq_2).is_ok());
        assert!(wrt.flush().is_ok());
        assert_eq!(directory.atomic_read(path_atomic).unwrap(), msg_atomic_2);
        assert_eq!(directory.atomic_read(path_seq).unwrap(), concat_helper(msg_seq, msg_seq_2));
    }

    fn concat_helper(a: &[u8], b: &[u8]) -> Vec<u8>{
        let mut concatenated = Vec::with_capacity(a.len()+b.len());
        concatenated.extend_from_slice(a);
        concatenated.extend_from_slice(b);
        concatenated
    }
}

