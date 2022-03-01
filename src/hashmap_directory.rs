use std::{path::{Path, PathBuf}, ops::Range, collections::HashMap, sync::{Arc, Mutex}, io::{BufWriter, Write}};

use tantivy::{Directory, directory::{error::{DeleteError, OpenReadError, OpenWriteError}, FileHandle, WritePtr, WatchCallback, WatchHandle, OwnedBytes, TerminatingWrite, AntiCallToken}, TantivyError, HasLen};


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
        let mut hash_map_directory = self.0.lock().unwrap();
        let buffer_pointer = hash_map_directory.entry(path.to_path_buf()).or_insert(HashMapFile(Arc::new(Mutex::new(Vec::new()))));        

        buffer_pointer.0.lock().unwrap().extend_from_slice(data);
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