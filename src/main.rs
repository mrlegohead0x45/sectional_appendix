use std::{collections::HashMap, io::Read, iter, ops::Deref};

use adobe_cmap_parser::{get_byte_mapping, get_unicode_map};
use anyhow::{Context, Error, Ok, Result, anyhow, bail};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use image::ExtendedColorType;
use pdf::{
    // content::Op::{self, TextDrawAdjusted},
    PdfError,
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
    let path = "London North Western (South) Sectional Appendix December 2025.pdf";
    let doc = FileOptions::cached()
        .open(path)
        .with_context(|| format!("Failed to open {:?}", path))?;

    let mut pages = doc.pages();

    let iol_fake_page_no = get_iol_page_no(&doc.resolver(), &mut pages)
        .context("Failed to find location of Index of Locations")?;
    println!("found iol page number {}", iol_fake_page_no);

    let iol_strings = get_iol_strings(&doc.resolver(), pages, iol_fake_page_no)
        .context("Failed to extract Index of Locations")?;
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
    //     extract_grouped_strings(doc.get_page(19)?, &doc.resolver())?
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

fn get_iol_strings(
    resolver: &impl Resolve,
    mut pages: impl Iterator<Item = Result<PageRc, PdfError>>,
    iol_fake_page_no: String,
) -> Result<Vec<String>, Error> {
    let mut iol_strings = Vec::new();

    for (page_no, page) in pages.by_ref().enumerate() {
        let page = page.with_context(|| format!("Failed to get page {}", page_no))?;

        let strings = extract_grouped_strings(page, resolver)
            .with_context(|| format!("Failed to extract strings on page {}", page_no))?;

        if *strings
            .last()
            .context("No strings found in page")
            .context("Failed to skip to Index of Locations")?
            == iol_fake_page_no
        {
            // println!("found iol");
            iol_strings = strings[5..=strings.len() - 3].to_vec();
            break;
        }
    }
    for (page_no, page) in pages.by_ref().enumerate() {
        let page = page.with_context(|| format!("Failed to get page {}", page_no))?;

        let strings = extract_grouped_strings(page, resolver)
            .with_context(|| format!("Failed to extract grouped strings on page {}", page_no))?;
        if strings[2] != "Location" {
            break;
        }

        iol_strings.append(&mut strings[4..=strings.len() - 3].to_vec())
    }
    Ok(iol_strings)
}

fn get_iol_page_no(
    resolver: &impl Resolve,
    pages: &mut impl Iterator<Item = Result<PageRc, PdfError>>,
) -> Result<String> {
    let mut iol_fake_page_no = "".to_string();
    for (page_no, page) in pages.by_ref().enumerate() {
        // {
        let page = page.with_context(|| format!("Failed to get page {}", page_no))?;

        let strings = extract_grouped_strings(page, resolver)
            .with_context(|| format!("Failed to extract grouped strings on page {}", page_no))?;

        if strings.contains(&"Table of Contents".to_string()) {
            let iol_idx: usize = strings
                .iter()
                .position(|s| s == "Index of Locations")
                .with_context(|| {
                    format!(
                        "Could not find Index of Locations in table of contents on page {}",
                        page_no
                    )
                })?;

            iol_fake_page_no = strings[iol_idx + 1].clone();
            break;
        };
        // Ok(())
        // }
        // .with_context(|| format!("Failed on page {}", page_no))?;
    }
    Ok(iol_fake_page_no)
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
        .map(|to| {
            println!("{:?}", to.text);
            to
        })
        .collect();

    // sort top to bottom left to right (pdfs have y increasing upwards)
    objs.sort_by_key(|t| (-t.y, t.x));

    let fonts = &page
        .resources()
        .context("Failed to get page resources")?
        .fonts;
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
        .map(|raw| {
            let o = unicodify(raw, fonts, resolver)
                .with_context(|| format!("Failed to unicodify text object {:?}", raw));
            dbg!(&o);
            o
        })
        .collect::<Result<Vec<UnicodeTextObject>>>()
        .context("Failed to unicodify text objects")?;

    let grouped = group_textobjects(unicodified)
        .into_iter()
        .filter_map(|to| {
            let t = to.text.trim().to_string();
            if !t.is_empty() { Some(t) } else { None }
        })
        .collect();

    Ok(grouped)
}

fn unicodify(
    raw: &RawTextObject,
    fonts: &HashMap<Name, Lazy<Font>>,
    resolver: &impl Resolve,
) -> Result<UnicodeTextObject> {
    let font = fonts
        .get(&raw.font_name)
        .context("Failed to get font")?
        .load(resolver)?;

    // println!("{:?}", font);
    // println!("{:?,}", font.load(resolver));
    let font_avg_width = match font.data {
        FontData::TrueType(TFont {
            font_descriptor: Some(FontDescriptor { avg_width, .. }),
            ..
        }) => avg_width,
        // _ => Err("Could not get font average glyph width"),
        _ => 0.521,
    };
    let text = match &font.to_unicode {
        None => Ok(String::from_utf8(raw.text.clone())
            .with_context(|| format!("Invalid UTF8 when no ToUnicode was found"))
            .with_context(|| format!("Font was {:?}", font))?),

        Some(u) => {
            let cmap_bytes = u.deref().data(resolver).context("Failed to get CMAP")?;
            let map: HashMap<u32, Vec<u8>> = get_unicode_map(&cmap_bytes)
                .map_err(Error::msg)
                .context("Failed to parse CMAP")?;
            let byte_mapping = get_byte_mapping(&cmap_bytes)
                .map_err(Error::msg)
                .context("Failed to parse CMAP")?;
            // dbg!(byte_mapping);
            let width = byte_mapping
                .codespace
                .first()
                .context("No codespace found in CMAP")?
                .width as usize
                + 1;
            let map: HashMap<Vec<u8>, &Vec<u8>> = HashMap::from_iter(map.iter().map(|(cid, v)| {
                (
                    {
                        let mut cid = vec![*cid as u8];
                        cid.splice(..0, iter::repeat(0u8).take(width - cid.len()));
                        cid
                    },
                    v,
                )
            }));
            // dbg!(&map);
            let text = String::from_utf16(
                &raw.text
                    .chunks(width)
                    .map(|cid| {
                        let mut cid = cid.to_vec();
                        cid.splice(..0, iter::repeat(0u8).take(width - cid.len()));
                        // .collect::<Vec<u8>>();
                        // dbg!(&cid);
                        let b = map.get(&cid);
                        match b {
                            None => {
                                dbg!(&cid);
                                println!("{}", String::from_utf8(cmap_bytes.to_vec())?);
                                bail!("Failed to get unicode for character {:?}", cid);
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

fn zlib_inflate(buf: &[u8]) -> Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(buf);
    let mut inflated = Vec::<u8>::new();
    z.read_to_end(&mut inflated)?;
    Ok(inflated)
}
