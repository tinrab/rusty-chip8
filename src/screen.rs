pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub struct Screen {
    pub pixels: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
}

impl Screen {
    pub fn new() -> Self {
        Self {
            pixels: [false; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize],
        }
    }

    pub fn toggle(&mut self, x: u8, y: u8) -> bool {
        let (x, y) = Self::clamp(x, y);
        let index = y * SCREEN_WIDTH as usize + x;
        let previous = self.pixels[index];
        self.pixels[index] = !previous;
        previous
    }

    pub fn clear(&mut self) {
        self.pixels = [false; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize];
    }

    pub fn fill(&mut self) {
        self.pixels = [true; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize];
    }

    pub fn clamp(x: u8, y: u8) -> (usize, usize) {
        let x = if x > SCREEN_WIDTH as u8 {
            x - SCREEN_WIDTH as u8
        } else if x < 0 {
            x + SCREEN_WIDTH as u8
        } else {
            x
        };
        let y = if y > SCREEN_HEIGHT as u8 {
            y - SCREEN_HEIGHT as u8
        } else if y < 0 {
            y + SCREEN_HEIGHT as u8
        } else {
            y
        };
        (x as usize, y as usize)
    }
}
