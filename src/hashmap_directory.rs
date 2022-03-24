use core::fmt;
use std::{path::{Path, PathBuf}, ops::Range, collections::{HashMap}, sync::{Arc, Mutex}, io::{BufWriter, Write}, marker::PhantomData, mem};

use tantivy::{Directory, directory::{error::{DeleteError, OpenReadError, OpenWriteError}, FileHandle, WritePtr, WatchCallback, WatchHandle, OwnedBytes, TerminatingWrite, AntiCallToken, self}, TantivyError, HasLen};

use rkyv::{Archive, Deserialize, Serialize};
use bytecheck::CheckBytes;

use core::fmt::Write as fmtWrite;

use crate::{log, log3};
use sha1::{Sha1, Digest};


#[derive(Debug, Archive, Serialize, Deserialize)]
// To use the safe API, you have to derive CheckBytes for the archived type
#[archive_attr(derive(CheckBytes, Debug))]
pub struct SerializableHashMapDirectory(HashMap<String,Vec<u8>>);

impl From<&HashMapDirectory> for SerializableHashMapDirectory {
    fn from(value: &HashMapDirectory) -> Self {
        let hashmap = value.0.lock().unwrap().iter().map(|(path, file)|{
            let vec = file.0.lock().unwrap().to_owned();
            let string_path = path.clone().into_os_string().into_string().unwrap();
            (string_path, vec)
        }).collect();
        SerializableHashMapDirectory(hashmap)
    }
}

impl Into<HashMapDirectory> for SerializableHashMapDirectory {
    fn into(self) -> HashMapDirectory {
        let hashmap = self.0.iter().map(|(path, file)|{
            let hashmapfile = HashMapFile(Arc::new(Mutex::new(file.to_owned())));
            let path = Path::new(path).to_owned();
            (path, hashmapfile)
        }).collect();
        HashMapDirectory(Arc::new(Mutex::new(hashmap)))
    }
}

#[derive(Debug, Clone)]
pub struct HashMapDirectory(Arc<Mutex<HashMap<PathBuf, HashMapFile>>>);

fn to_hex_string(a: &[u8]) -> String{
    let mut s = String::with_capacity(2 * a.len());
    for byte in a {
        write!(s, "{:02X}", byte).unwrap();
    }
    s
}





impl HashMapDirectory {
    pub fn new()-> Self {
        HashMapDirectory(Arc::new(Mutex::new(HashMap::new())))
    }
    
    pub fn agregate(&mut self, directory: HashMapDirectory){
        
        let remote_directory_inner_map = mem::replace(&mut *directory.0.lock().unwrap(), HashMap::new());
        let mut self_inner_map = self.0.lock().unwrap();

        for (path, file) in remote_directory_inner_map {
            if path != Path::new("meta.json") {
                self_inner_map.insert(path, file);
            }
        }
    }

    pub fn remove_directory(&mut self, directory: HashMapDirectory){
        let remote_directory_inner_map = directory.0.lock().unwrap();
        let mut self_inner_map = self.0.lock().unwrap();

        for (path, _) in remote_directory_inner_map.iter() {
            if path != Path::new("meta.json") {
                self_inner_map.remove(path);
            }
        }
    }

    pub fn summary(&self){
        log("----- Directory: FILE LIST","");
        for (path, file) in self.0.lock().unwrap().iter() {
            if path == Path::new("meta.json") {
                let file_content = file.0.lock().unwrap();
                let file_str = std::str::from_utf8(&file_content).unwrap();
                log("--------------------------: meta.json", file_str);
            }else{
                let mut hasher = Sha1::new();
                let content = file.0.lock().unwrap();
                hasher.update(&*content);
                let hex = to_hex_string(&hasher.finalize());
                log3("--------------------------: file", path.to_str().unwrap(), &hex);
            }
        }
    }
    
    fn get_meta(&self)-> HashMapFile{
        let lock = self.0.lock().unwrap();
        let file = lock.get(Path::new("meta.json")).expect("There must be a meta.json file in a directory").clone();
        file
    }
}

impl Directory for HashMapDirectory {
    fn get_file_handle(
        &self, 
        path: &Path
    ) -> Result<Box<dyn FileHandle>, OpenReadError>{

        log("----- Directory: get_file_handle", path.to_str().unwrap());
        match self.0.lock().unwrap().get(path) {
            None => Err(OpenReadError::FileDoesNotExist(path.into())),
            Some(buffer_pointer) => {
                Ok(Box::new(buffer_pointer.clone()))
            }
        }
    }
    
    fn delete(&self, path: &Path) -> Result<(), DeleteError>{
        log("----- Directory: delete", path.to_str().unwrap());
        let ret = match self.0.lock().unwrap().remove(path) {
            None => Err(DeleteError::FileDoesNotExist(path.into())),
            Some(_) => {
                Ok(())
            }
        };
        
        ret
    }
    
    fn exists(&self, path: &Path) -> Result<bool, OpenReadError>{
        log("----- Directory: exists", path.to_str().unwrap());
        Ok(self.0.lock().unwrap().contains_key(path))
    }
    
    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError>{
        log("----- Directory: open_write", path.to_str().unwrap());
        let mut hash_map_directory = self.0.lock().unwrap();
        let buffer_pointer = hash_map_directory.entry(path.to_path_buf()).or_insert(HashMapFile(Arc::new(Mutex::new(Vec::new()))));        
        Ok(BufWriter::new(Box::new(buffer_pointer.clone())))
    }
    
    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError>{
        log("----- Directory: atomic_read", path.to_str().unwrap());
        match self.0.lock().unwrap().get(path) {
            None => Err(OpenReadError::FileDoesNotExist(path.into())),
            Some(buffer_pointer) => {
                Ok(buffer_pointer.0.lock().unwrap().clone())
            }
        }
    }
    
    fn atomic_write(&self, path: &Path, data: &[u8]) -> std::io::Result<()>{
        log("----- Directory: atomic_write", path.to_str().unwrap());
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
        log("----- Directory: watch", "");
        Ok(WatchHandle::empty())
    }
    
    fn sync_directory(&self) -> Result<(), std::io::Error> {
        log("----- Directory: sync_directory", "");
        Ok(()) 
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

