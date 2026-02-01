use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use image::ExtendedColorType;
use pdf::{
    file::FileOptions,
    font::{FontData, FontDescriptor, FontType, TFont},
    object::{PageRc, Resolve},
};

use crate::page::{TextObject, text_objects};
// use lopdf::{Document, Object, StringFormat};

mod images;
mod indices;
mod page;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let doc = Document::load("London North Western (South) Sectional Appendix December 2025.pdf")?;
    let doc = FileOptions::cached()
        .open("London North Western (South) Sectional Appendix December 2025.pdf")?;

    let mut pages = doc.pages();

    let mut iol_fake_page_no = "".to_string();
    while let Some(page) = pages.next() {
        let page = page?;

        let strings = extract_grouped_strings(page, doc.resolver())?;

        if strings.contains(&"Table of Contents".to_string()) {
            let iol_idx = strings
                .iter()
                .position(|s| s == "Index of Locations")
                .ok_or("Could not find Index of Locations in table of contents")?;

            iol_fake_page_no = strings[iol_idx + 1].clone();
            break;
        }
        // for obj in grouped {
        //     println!("{:?}", obj);
        // }
    }
    println!("found iol page number {}", iol_fake_page_no);

    let mut iol_strings = Vec::new();

    while let Some(page) = pages.next() {
        let page = page?;

        let strings = extract_grouped_strings(page, doc.resolver())?;

        if *strings.last().ok_or("No strings found in page")? == iol_fake_page_no {
            println!("found iol");
            iol_strings = strings[5..=strings.len() - 3].to_vec();
            break;
        }
    }

    while let Some(page) = pages.next() {
        let page = page?;

        let strings = extract_grouped_strings(page, doc.resolver())?;
        if strings[2] != "Location" {
            break;
        }

        iol_strings.append(&mut strings[4..=strings.len() - 3].to_vec())
    }
    println!("{:?}", iol_strings);

    // println!("{:#?}", font);
    // match font.data {
    //     FontType::TrueType => {}
    //     _ => println!("other type of font found"),
    // }
    // println!(
    //     "{:?}",
    //     font.widths(&doc.resolver())?
    //         .ok_or("Could not get font widths")?
    // );
    // println!("{:?}", font.encoding().ok_or("Could not get font encoding"));
    // println!("{:?}", font.d)
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

/// Attempt to group adjacent textobjects forming a single text block into one
fn group_textobjects(objs: Vec<TextObject>, avg_width: f32) -> Vec<TextObject> {
    let mut grouped = Vec::new();

    let mut acc = None;
    for obj in objs {
        match acc {
            None => acc = Some(obj),
            Some(ref acc_obj) => {
                let expected_width =
                    (acc_obj.text.len() as f32) * acc_obj.font_size * avg_width / 1000.0;

                // close enough
                let diff_x = (*obj.x - *(acc_obj.x + expected_width)).abs();
                let diff_y = (*obj.y - *acc_obj.y).abs();
                // println!("{:?diff_x}")
                // dbg!(diff_x);
                // dbg!(diff_y);
                // dbg!(acc_obj);
                // dbg!(&obj);

                if diff_x < 22.0 && diff_y < 0.05 {
                    // println!("grouped {:?} and {:?}", &acc_obj.text, obj.text);
                    acc = Some(TextObject {
                        text: acc_obj.text.clone() + &obj.text,
                        ..acc_obj.clone()
                    });
                } else {
                    grouped.push(acc_obj.clone());
                    acc = Some(obj);
                }
            }
        }
    }

    grouped
}

fn extract_grouped_strings(
    page: PageRc,
    resolver: impl Resolve,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let ops = &page
        .contents
        .as_ref()
        .ok_or("Couldn't get page contents")?
        .operations(&resolver)?;
    let mut objs: Vec<TextObject> = text_objects(ops).collect();

    // sort top to bottom left to right (pdfs have y increasing upwards)
    objs.sort_by_key(|t| (-t.y, t.x));

    let font = page
        .resources()?
        .fonts
        .get(&objs[0].font_name)
        .ok_or("Could not get font")?
        .load(&resolver)?;

    let avg_width = match font.data {
        FontData::TrueType(TFont {
            font_descriptor: Some(FontDescriptor { avg_width, .. }),
            ..
        }) => Ok(avg_width),
        _ => Err("Could not get font average glyph width"),
    }?;

    let grouped = group_textobjects(objs, avg_width)
        .into_iter()
        .filter_map(|to| {
            let t = to.text.trim().to_string();
            if !t.is_empty() { Some(t) } else { None }
        })
        .collect();

    Ok(grouped)
}

// enum Stage {
//     SeekingFirstToc,
//     SeekingIndexOfLocations,
//     Extracting
// }
