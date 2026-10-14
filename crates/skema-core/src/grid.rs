use serde::{Deserialize, Serialize};

/// Fixed coordinate system for a Diagram (wire: grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Grid;

/// Integer cell coordinates on the Grid (wire: gridPosition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPosition {
    pub x: i64,
    pub y: i64,
}

impl GridPosition {
    pub fn cell_key(self) -> (i64, i64) {
        (self.x, self.y)
    }
}
