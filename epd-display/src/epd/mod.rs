macro_rules! bitmap_buffer_type {
    ($width:expr, $height:expr) => {
        crate::epd::bitmap_buffer::BitmapBuffer<$width, $height, { ($width + 7) / 8 * $height }>
    };
}

macro_rules! bitmap_buffer {
    ($width:expr, $height:expr) => {
        crate::epd::bitmap_buffer::BitmapBuffer::<$width, $height, { ($width + 7) / 8 * $height }>::new()
    };
}

pub mod display_orientation;
pub mod display_spi;
pub mod epd_2in66b;
pub mod three_color;

mod bitmap_buffer;
mod epd_spi;
