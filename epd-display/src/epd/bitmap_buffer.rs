pub struct BitmapBuffer<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize> {
    pub buffer: [u8; BUFFER_SIZE],
}

impl<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize>
    BitmapBuffer<WIDTH, HEIGHT, BUFFER_SIZE>
{
    pub fn new() -> Self {
        BitmapBuffer {
            buffer: [0x00; BUFFER_SIZE],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize) {
        if x < WIDTH && y < HEIGHT {
            let byte_index = byte_index(WIDTH, x, y);
            let bit_index = 7 - x % 8;
            self.buffer[byte_index] |= 1 << bit_index;
        }
    }

    pub fn clear_pixel(&mut self, x: usize, y: usize) {
        if x < WIDTH && y < HEIGHT {
            let byte_index = byte_index(WIDTH, x, y);
            let bit_index = 7 - x % 8;
            self.buffer[byte_index] &= !(1 << bit_index);
        }
    }

    pub fn fill(&mut self, value: u8) {
        for i in 0..BUFFER_SIZE {
            self.buffer[i] = value;
        }
    }
}

const fn byte_index(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) / 8
}
