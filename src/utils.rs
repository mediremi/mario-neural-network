#[derive(Copy, Clone)]
pub enum Tile {
    Nothing = 0,
    Block = 1,
    Enemy = 2,
    Mario = 3,
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Nothing
    }
}

// How many blocks NN sees to left, right, top and bottom of Mario
pub const VIEW_SIZE: i32 = 6;

pub const SCREEN_SIZE: usize = (VIEW_SIZE as usize * 2) + 1;
// Each block in SMB is a 'megatile'. Each megatile consists of four 8x8 tiles.
// The NN sees the screen as a 13x13 square of megatiles.
pub type Screen = [[Tile; SCREEN_SIZE]; SCREEN_SIZE];
