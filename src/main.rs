use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use image::ExtendedColorType;
use lopdf::{Document, Object, StringFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = Document::load("London North Western (South) Sectional Appendix December 2025.pdf")?;

    let pages = doc.get_pages();
    let page_130_id = pages.get(&130).ok_or("couldn't get page 130 id")?;
    let page_130 = doc
        .objects
        .get(page_130_id)
        .ok_or("couldn't get page 130")?;

    // std::fs::write("page_130.txt", format!("{:?}", page_130))?;
    // write!()

    let page_130_images = doc.get_page_images(*page_130_id)?;

    let img = &page_130_images[0];

    let mut z = ZlibDecoder::new(img.content);
    let mut inflated = Vec::<u8>::new();
    let x = z.read_to_end(&mut inflated)?;
    println!("{}", x);

    // println!("inflated len {}", inflated.len());

    image::save_buffer(
        "MD101-001.png",
        &inflated,
        img.width.try_into()?,
        img.height.try_into()?,
        ExtendedColorType::Rgb8,
    )?;

    // println!("Hello, world!");

    Ok(())
}
