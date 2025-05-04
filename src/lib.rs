#![no_std]

mod epd;
mod bitmap_buffer;

pub mod color;
pub mod display_orientation;
pub mod display_spi;
pub mod epd_2in66b;

#[macro_export]
macro_rules! BitmapBufferType {
    ($width:expr, $height:expr) => {
        crate::bitmap_buffer::BitmapBuffer<$width, $height, { ($width + 7) / 8 * $height }>
    };
}

#[macro_export]
macro_rules! bitmap_buffer {
    ($width:expr, $height:expr) => {
        crate::bitmap_buffer::BitmapBuffer::<$width, $height, { ($width + 7) / 8 * $height }>::new()
    };
}

