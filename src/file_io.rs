use std::io;
use std::path::Path;

use crate::state::{GRID_COLS, GRID_ROWS};

/// Read a CSV file into a 2D grid of strings
pub fn read_csv(path: &Path) -> io::Result<Vec<Vec<String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)?;

    let mut cells: Vec<Vec<String>> = (0..GRID_ROWS)
        .map(|_| (0..GRID_COLS).map(|_| String::new()).collect())
        .collect();

    for (row_idx, result) in reader.records().enumerate() {
        if row_idx >= GRID_ROWS {
            break;
        }
        let record = result?;
        for (col_idx, field) in record.iter().enumerate() {
            if col_idx >= GRID_COLS {
                break;
            }
            cells[row_idx][col_idx] = field.to_string();
        }
    }

    Ok(cells)
}

/// Write a 2D grid of strings to a CSV file
pub fn write_csv(path: &Path, cells: &[Vec<String>]) -> io::Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(path)?;

    // Find the actual used bounds to avoid writing empty trailing rows/cols
    let (max_row, max_col) = find_used_bounds(cells);

    for row in 0..=max_row {
        let row_data: Vec<&str> = (0..=max_col)
            .map(|col| cells[row][col].as_str())
            .collect();
        writer.write_record(&row_data)?;
    }

    writer.flush()?;
    Ok(())
}

/// Find the bounds of non-empty cells
fn find_used_bounds(cells: &[Vec<String>]) -> (usize, usize) {
    let mut max_row = 0;
    let mut max_col = 0;

    for (row_idx, row) in cells.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                max_row = max_row.max(row_idx);
                max_col = max_col.max(col_idx);
            }
        }
    }

    (max_row, max_col)
}
