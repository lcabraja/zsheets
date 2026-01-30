pub const GRID_ROWS: usize = 100;
pub const GRID_COLS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPosition {
    pub row: usize,
    pub col: usize,
}

impl CellPosition {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// Convert to Excel-style cell reference (A1, B5, etc.)
    pub fn to_reference(&self) -> String {
        let col_letter = Self::col_to_letter(self.col);
        format!("{}{}", col_letter, self.row + 1)
    }

    fn col_to_letter(col: usize) -> String {
        let mut result = String::new();
        let mut n = col;
        loop {
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Edit,
}
