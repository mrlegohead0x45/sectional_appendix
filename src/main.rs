use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use image::ExtendedColorType;
use pdf::file::FileOptions;

use crate::page::{TextObject, text_objects};
// use lopdf::{Document, Object, StringFormat};

mod images;
mod indices;
mod page;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let doc = Document::load("London North Western (South) Sectional Appendix December 2025.pdf")?;
    let doc = FileOptions::cached()
        .open("London North Western (South) Sectional Appendix December 2025.pdf")?;

    let pages = doc.pages();
    let page = doc.get_page(3 - 1)?;
    let ops = &page
        .contents
        .as_ref()
        .ok_or("Couldn't get page contents")?
        .operations(&doc.resolver())?;
    let mut objs: Vec<TextObject> = text_objects(ops).collect();

    // sort top to bottom left to right (pdfs have y increasing upwards)
    objs.sort_by_key(|t| (-t.y, t.x));
    // objs.reverse();

    for obj in objs.iter() {
        println!("{:?}", obj);
    }
    // let objs =

    // let pages = doc.get_pages();
    // let page_id = pages.get(&92).ok_or("couldn't get page id")?;
    // let page = doc.objects.get(page_id).ok_or("couldn't get page")?;

    // // std::fs::write("page_92.txt", format!("{:?}", page_130))?;
    // let r = match page {
    //     Object::Dictionary(d) => match d.get("Contents".as_bytes())? {
    //         Object::Reference(obj_id) => obj_id,
    //         _ => todo!(),
    //     },
    //     _ => todo!(),
    // };

    // let page_contents = doc.objects.get(r).ok_or("err")?;

    // println!("{:?}", page_contents);
    // println!("{}", page_contents.enum_variant());

    // let m = match page_contents {
    //     Object::Stream(s) => zlib_inflate(&s.content)?,
    //     _ => todo!(),
    // };

    // let doc1 = Document::load_mem(&m)?;
    // let mut b = String::new();
    // for (_, o) in doc1.objects {
    //     match o {
    //         Object::String(s, StringFormat::Literal) => {
    //             b.push_str(&String::from_utf8(s)?);
    //         }
    //         _ => {}
    //     }
    // }
    // std::fs::write("page_92_text.txt", doc.extract_text(&[92])?)?;

    // write!()

    // let page_130_images = doc.get_page_images(*page_130_id)?;

    // let img = &page_130_images[0];

    // let mut z = ZlibDecoder::new(img.content);
    // let mut inflated = Vec::<u8>::new();
    // let x = z.read_to_end(&mut inflated)?;
    // println!("{}", x);

    // // println!("inflated len {}", inflated.len());

    // image::save_buffer(
    //     "MD101-001.png",
    //     &inflated,
    //     img.width.try_into()?,
    //     img.height.try_into()?,
    //     ExtendedColorType::Rgb8,
    // )?;

    // println!("Hello, world!");

    // let i = extract_indices(&doc, (92, 105));
    // println!("{:?}", i);

    Ok(())
}

fn zlib_inflate(buf: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut z = ZlibDecoder::new(buf);
    let mut inflated = Vec::<u8>::new();
    z.read_to_end(&mut inflated)?;
    Ok(inflated)
}
