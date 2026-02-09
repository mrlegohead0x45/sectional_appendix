use std::io::Read;

use anyhow::{Context, Ok, Result};
use flate2::read::ZlibDecoder;
use pdf::{
    // content::Op::{self, TextDrawAdjusted},
    file::FileOptions,
};

use crate::{
    diagrams::extract_image,
    iol::{get_iol_page_no, get_iol_strings},
};
// use crate::iol::{get}
// use lopdf::{Document, Object, StringFormat};

mod diagrams;
mod indices;
mod iol;
mod strings;
mod text_object;
mod unicode;

fn main() -> Result<()> {
    let start = std::time::Instant::now();
    // let doc = Document::load("London North Western (South) Sectional Appendix December 2025.pdf")?;
    let path = "London North Western (South) Sectional Appendix December 2025.pdf";
    let doc = FileOptions::cached()
        .open(path)
        .with_context(|| format!("Failed to open {:?}", path))?;

    let mut pages = doc.pages();

    let iol_fake_page_no = get_iol_page_no(&doc.resolver(), &mut pages)
        .context("Failed to find location of Index of Locations")?;
    // println!("found iol page number {}", iol_fake_page_no);

    let iol_strings = get_iol_strings(&doc.resolver(), pages, iol_fake_page_no)
        .context("Failed to extract Index of Locations")?;
    let end = std::time::Instant::now();
    // println!("{:?}", end - start);

    // println!("{:#?}", iol_strings);

    extract_image(doc.get_page(130 - 1)?, &doc.resolver(), "MD101-001.png")?;

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

fn zlib_inflate(buf: &[u8]) -> Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(buf);
    let mut inflated = Vec::<u8>::new();
    z.read_to_end(&mut inflated)?;
    Ok(inflated)
}
