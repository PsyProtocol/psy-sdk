use std::{cell::UnsafeCell, collections::HashMap, path::PathBuf};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub usize);

#[derive(Debug)]
pub struct FileResolver {
    file_contents: UnsafeCell<Vec<String>>,
    file_ids: UnsafeCell<HashMap<PathBuf, FileId>>,
}

unsafe impl Sync for FileResolver {}

impl FileResolver {
    pub fn new() -> Self {
        Self {
            file_contents: UnsafeCell::new(Vec::with_capacity(20)),
            file_ids: UnsafeCell::new(HashMap::new()),
        }
    }

    pub fn resolve_id(&self, file_path: &PathBuf) -> Option<&FileId> {
        unsafe {
            let file_ids = &mut *self.file_ids.get();
            file_ids.get(file_path)
        }
    }

    pub fn resolve_file(&self, file_path: PathBuf) -> std::io::Result<FileId> {
        unsafe {
            let file_ids = &mut *self.file_ids.get();
            if let Some(&file_id) = file_ids.get(&file_path) {
                return Ok(file_id);
            }

            let file_contents = &mut *self.file_contents.get();
            let file_id = FileId(file_contents.len());
            file_contents.push(std::fs::read_to_string(&file_path)?);
            file_ids.insert(file_path, file_id);
            Ok(file_id)
        }
    }

    pub fn resolve_content(&self, file_id: &FileId) -> Option<&str> {
        unsafe {
            let file_contents = &*self.file_contents.get();
            file_contents.get(file_id.0).map(|s| s.as_str())
        }
    }
}

impl Default for FileResolver {
    fn default() -> Self {
        Self::new()
    }
}
