#[derive(Copy, Clone)]
pub enum Tile {
    Nothing = 0,
    Block = 1,
    Enemy = 2,
    Mario = 3,
}

pub type Screen = [[Tile; 32]; 30];
