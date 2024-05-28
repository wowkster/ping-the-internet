use image::{ImageBuffer, Rgb};
use ping_the_internet::{
    file::read_slash_16,
    ping::PingResult,
    subnet::{Subnet, SubnetMask},
};

#[tokio::main]
async fn main() {
    let results = read_slash_16(Subnet::new([8, 0, 0, 0].into(), SubnetMask::Slash16))
        .await
        .expect("Failed to read file")
        .expect("Subnet not found");

    let mut map = ImageBuffer::new(256, 256);

    for i in 0..=u16::MAX {
        let (x, y) = hilbert_curve::convert_1d_to_2d(i as usize, 256);

        let a = (i / 256) as usize;
        let b = (i % 256) as usize;

        let pixel_color: [u8; 3] = match &results[a] {
            Some(slash_24) => match slash_24[b] {
                PingResult::Success(_) => [0x40, 0xFF, 0x40],
                PingResult::Timeout => [0xA3, 0xB3, 0xC0],
                PingResult::Error => [0xFF, 0x50, 0x50]
            },
            None => [0x50, 0x50, 0x50],
        };

        map.put_pixel(x as u32, y as u32, Rgb(pixel_color));
    }

    map.save("output.png").expect("Failed to save output image");
}
