use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state::{GRID_COLS, GRID_ROWS};
use crate::grid::{DEFAULT_CELL_WIDTH, DEFAULT_CELL_HEIGHT};

/// Metadata for spreadsheet dimensions and settings
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SpreadsheetMetadata {
    pub column_widths: Option<Vec<f32>>,
    pub row_heights: Option<Vec<f32>>,
}

impl SpreadsheetMetadata {
    /// Get the metadata file path for a given CSV file
    pub fn metadata_path(csv_path: &Path) -> std::path::PathBuf {
        let mut path = csv_path.to_path_buf();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("spreadsheet.csv");
        path.set_file_name(format!("{}.zsheets", file_name));
        path
    }

    /// Load metadata from a CSV file's companion metadata file
    pub fn load(csv_path: &Path) -> io::Result<Self> {
        let meta_path = Self::metadata_path(csv_path);
        if !meta_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&meta_path)?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Save metadata to a CSV file's companion metadata file
    pub fn save(&self, csv_path: &Path) -> io::Result<()> {
        let meta_path = Self::metadata_path(csv_path);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(&meta_path, content)
    }

    /// Get column widths, filling with defaults if needed
    pub fn get_column_widths(&self) -> Vec<f32> {
        let mut widths = self.column_widths.clone().unwrap_or_default();
        widths.resize(GRID_COLS, DEFAULT_CELL_WIDTH);
        widths
    }

    /// Get row heights, filling with defaults if needed
    pub fn get_row_heights(&self) -> Vec<f32> {
        let mut heights = self.row_heights.clone().unwrap_or_default();
        heights.resize(GRID_ROWS, DEFAULT_CELL_HEIGHT);
        heights
    }
}
