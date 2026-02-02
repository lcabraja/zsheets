use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileState {
    pub current_path: Option<PathBuf>,
    pub is_dirty: bool,
    pub is_read_only: bool,
}

impl Default for FileState {
    fn default() -> Self {
        Self::new()
    }
}

impl FileState {
    pub fn new() -> Self {
        Self {
            current_path: None,
            is_dirty: false,
            is_read_only: false,
        }
    }

    pub fn file_name(&self) -> String {
        self.current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "[No Name]".to_string())
    }

    pub fn mark_dirty(&mut self) {
        if !self.is_read_only {
            self.is_dirty = true;
        }
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.current_path = Some(path);
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.is_read_only = read_only;
    }
}
