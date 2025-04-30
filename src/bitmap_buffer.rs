
pub struct BitmapBuffer<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize> {
    pub buffer: [u8; BUFFER_SIZE],
}

impl<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize> BitmapBuffer<WIDTH, HEIGHT, BUFFER_SIZE> {
    pub fn new() -> Self {
        BitmapBuffer {
            buffer: [0; BUFFER_SIZE],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize) {
        if x < WIDTH && y < HEIGHT {
            let byte_index = byte_index(WIDTH, x, y);
            let bit_index = x % 8;
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

    pub fn set_byte(&mut self, byte_index: usize, value: u8) {
        if byte_index < BUFFER_SIZE {
            self.buffer[byte_index] = value;
        }
    }

    pub fn clear(&mut self) {
        for i in 0..BUFFER_SIZE {
            self.buffer[i] = 0;
        }
    }
}

const fn byte_index(width: usize, x:usize, y:usize) -> usize {
    (y * width + x) / 8
}

#[macro_export]
macro_rules! BitmapBufferType {
    ($width:literal, $height:literal) => {
        bitmap_buffer::BitmapBuffer::<$width, $height, {($width+7)/8*$height}>
    };
}

#[macro_export]
macro_rules! bitmap_buffer {
    ($width:literal, $height:literal) => {
        bitmap_buffer::BitmapBuffer::<$width, $height, {($width+7)/8*$height}>::new()
    };
}
