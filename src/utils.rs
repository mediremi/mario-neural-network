#[derive(Copy, Clone)]
pub enum Tile {
    Nothing = 0,
    Block = 1,
    Enemy = 2,
    Mario = 3,
}

impl Tile {
    fn as_nn_input(&self) -> f64 {
        use self::Tile::*;

        match self {
            Nothing | Mario => 0.0,
            Block => 1.0,
            Enemy => -1.0
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Nothing
    }
}

// Each block in SMB is a 'megatile'. Each megatile consists of four 8x8 tiles.
// The NN sees the screen as a 13x13 square of megatiles.
pub type Screen = [[Tile; 13]; 13];
