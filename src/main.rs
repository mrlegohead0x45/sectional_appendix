use std::{collections::HashMap, io::Read};

use adobe_cmap_parser::get_unicode_map;
use anyhow::{Context, Error, Ok, Result, anyhow};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use image::ExtendedColorType;
use pdf::{
    // content::Op::{self, TextDrawAdjusted},
    file::FileOptions,
    font::{Font, FontData, FontDescriptor, TFont},
    object::{Lazy, PageRc, Resolve},
    primitive::Name,
};

use crate::page::{RawTextObject, UnicodeTextObject, text_objects};
// use lopdf::{Document, Object, StringFormat};

mod images;
mod indices;
mod page;

fn main() -> Result<()> {
    // let doc = Document::load("London North Western (South) Sectional Appendix December 2025.pdf")?;
    let path = "London North Western (South) Sectional Appendix December 2025.pf";
    let doc = FileOptions::cached()
        .open(path)
        .with_context(|| format!("Failed to open {:?}", path))?;

    let mut pages = doc.pages();

    let mut iol_fake_page_no = "".to_string();
    for page in pages.by_ref() {
        let page = page?;

        let strings = extract_grouped_strings(page, &doc.resolver())?;

        if strings.contains(&"Table of Contents".to_string()) {
            let iol_idx: usize = strings
                .iter()
                .position(|s| s == "Index of Locations")
                .context("Could not find Index of Locations in table of contents")?;

            iol_fake_page_no = strings[iol_idx + 1].clone();
            break;
        }
    }
    println!("found iol page number {}", iol_fake_page_no);

    let mut iol_strings = Vec::new();

    for page in pages.by_ref() {
        let page = page?;

        let strings = extract_grouped_strings(page, &doc.resolver())?;

        if *strings.last().context("No strings found in page")? == iol_fake_page_no {
            println!("found iol");
            iol_strings = strings[5..=strings.len() - 3].to_vec();
            break;
        }
    }

    for page in pages.by_ref() {
        let page = page?;

        let strings = extract_grouped_strings(page, &doc.resolver())?;
        if strings[2] != "Location" {
            break;
        }

        iol_strings.append(&mut strings[4..=strings.len() - 3].to_vec())
    }
    println!("{:?}", iol_strings);

    // image::save_buffer(
    //     "MD101-001.png",
    //     &inflated,
    //     img.width.try_into()?,
    //     img.height.try_into()?,
    //     ExtendedColorType::Rgb8,
    // )?;

    // println!(
    //     "{:?}",
    //     extract_grouped_strings(doc.get_page(99 - 1)?, &doc.resolver())?[9]
    // );

    // let page = doc.get_page(99 - 1)?;
    // let ops = &page
    //     .contents
    //     .as_ref()
    //     .ok_or("Couldn't get page contents")?
    //     .operations(&doc.resolver())?;
    // println!(
    //     "{:#?}",
    //     ops.iter()
    //         // .filter(|op| match op {
    //         //     TextDrawAdjusted { .. } => true,
    //         //     _ => false,
    //         // })
    //         .collect::<Vec<&Op>>()
    // );

    Ok(())
}

fn zlib_inflate(buf: &[u8]) -> Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(buf);
    let mut inflated = Vec::<u8>::new();
    z.read_to_end(&mut inflated)?;
    Ok(inflated)
}

/// Attempt to group adjacent textobjects forming a single text block into one
fn group_textobjects(objs: Vec<UnicodeTextObject>) -> Vec<UnicodeTextObject> {
    let mut grouped = Vec::new();

    let mut acc = None;
    for obj in objs {
        // println!("{:?}", obj.text);
        match acc {
            None => acc = Some(obj),
            Some(ref acc_obj) => {
                // don't try and merge text objects with different fonts
                if acc_obj.font_name != obj.font_name {
                    grouped.push(acc_obj.clone());
                    acc = Some(obj);
                    continue;
                };

                let expected_width =
                    (acc_obj.text.len() as f32) * acc_obj.font_size * obj.font_avg_width / 1000.0;

                // close enough
                let diff_x = (*obj.x - *(acc_obj.x + expected_width)).abs();
                let diff_y = (*obj.y - *acc_obj.y).abs();

                if diff_x < 22.0 && diff_y < 0.05 {
                    acc = Some(UnicodeTextObject {
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

fn extract_grouped_strings(page: PageRc, resolver: &impl Resolve) -> Result<Vec<String>> {
    let ops = &page
        .contents
        .as_ref()
        .context("Failed to get page contents")?
        .operations(resolver)?;
    let mut objs: Vec<RawTextObject> = text_objects(ops)
        // .map(|to| {
        //     println!("{:?}", to.text);
        //     to
        // })
        .collect();

    // sort top to bottom left to right (pdfs have y increasing upwards)
    objs.sort_by_key(|t| (-t.y, t.x));

    let fonts = &page.resources()?.fonts;
    // .get(&objs[0].font_name)
    // .ok_or("Could not get font")?
    // .load(&resolver)?;

    // let avg_width = match font.data {
    //     FontData::TrueType(TFont {
    //         font_descriptor: Some(FontDescriptor { avg_width, .. }),
    //         ..
    //     }) => Ok(avg_width),
    //     _ => Err("Could not get font average glyph width"),
    // }?;

    let unicodified = objs
        .iter()
        .map(|raw| unicodify(raw, fonts, resolver))
        .collect::<Result<Vec<UnicodeTextObject>>>()?;

    let grouped = group_textobjects(unicodified)
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

fn unicodify(
    raw: &RawTextObject,
    fonts: &HashMap<Name, Lazy<Font>>,
    resolver: &impl Resolve,
) -> Result<UnicodeTextObject> {
    let font = fonts
        .get(&raw.font_name)
        .context("Failed get font")?
        .load(resolver)?;

    // println!("{:?}", font);
    // println!("{:?}", font.load(resolver));
    let font_avg_width = match font.data {
        FontData::TrueType(TFont {
            font_descriptor: Some(FontDescriptor { avg_width, .. }),
            ..
        }) => avg_width,
        // _ => Err("Could not get font average glyph width"),
        _ => 0.521,
    };
    let text = match &font.to_unicode {
        None => Ok(String::from_utf8(raw.text.clone())?),
        Some(u) => {
            let map: HashMap<u32, Vec<u8>> = get_unicode_map(&u.data().data(resolver)?)
                .map_err(Error::msg)
                .context("Failed to parse CMAP")?;
            let map: HashMap<Vec<u8>, &Vec<u8>> =
                HashMap::from_iter(map.iter().map(|(cid, v)| (vec![0, *cid as u8], v)));
            let text = String::from_utf16(
                &raw.text
                    .chunks(2)
                    .map(|cid| {
                        let b = map.get(cid);
                        match b {
                            None => {
                                return Err(anyhow!(
                                    "Failed to get unicode for character {:?}",
                                    cid
                                ));
                            }
                            Some(b) => {
                                return Ok(0x0100 * b[0] as u16 + b[1] as u16);
                            }
                        };
                        // .expect("no unicode mapping found for character");
                    })
                    .collect::<Result<Vec<u16>>>()
                    .with_context(|| format!("Failed to decode bytes {:?}", &raw.text))?,
            )?;
            // todo!()
            Ok(text)
        }
    }?;
    Ok(UnicodeTextObject {
        x: raw.x,
        y: raw.y,
        font_size: raw.font_size,
        font_name: raw.font_name.clone(),
        font_avg_width,
        text,
    })
    // todo!()
}
