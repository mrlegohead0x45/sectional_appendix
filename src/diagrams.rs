use std::ops::Deref;
use std::path::Path;

use anyhow::{Context, Result};
use image::ExtendedColorType;
use pdf::{
    PdfError,
    content::Op,
    object::{PageRc, Resolve, XObject},
};

use crate::strings::extract_grouped_strings;

fn get_diagrams_location(
    mut pages: impl Iterator<Item = Result<PageRc, PdfError>>,
    resolver: &impl Resolve,
) -> Result<String> {
    let mut diagrams_fake_page_no = "".to_string();
    for (page_no, page) in pages.by_ref().enumerate() {
        // {
        let page = page.with_context(|| format!("Failed to get page {}", page_no))?;

        let strings = extract_grouped_strings(page, resolver)
            .with_context(|| format!("Failed to extract grouped strings on page {}", page_no))?;

        if strings.contains(&"Table of Contents".to_string()) {
            let diags_idx: usize = strings
                .iter()
                .position(|s| s == "Table A Diagrams")
                .with_context(|| {
                    format!(
                        "Could not find Table A Diagrams in table of contents on page {}",
                        page_no
                    )
                })?;

            diagrams_fake_page_no = strings[diags_idx + 1].clone();
            break;
        };
        // Ok(())
        // }
        // .with_context(|| format!("Failed on page {}", page_no))?;
    }
    Ok(diagrams_fake_page_no)
}

fn get_diagrams_toc(
    mut pages: impl Iterator<Item = Result<PageRc, PdfError>>,
    resolver: &impl Resolve,
    diagrams_fake_page_no: String,
) -> Result<Vec<(String, String)>> {
    todo!()
}

pub(crate) fn extract_image(
    page: PageRc,
    resolver: &impl Resolve,
    dest: impl AsRef<Path>,
) -> Result<()> {
    // let a = std::fs::File::open(path)
    let ops = &page
        .contents
        .as_ref()
        .context("Failed to get page contents")?
        .operations(resolver)?;

    for op in ops {
        match op {
            Op::InlineImage { image } => println!("{:?}", image),
            Op::XObject { name } => {
                let xobjects = &page
                    .resources()
                    .context("Couldn't get page resources while extracting image")?
                    .xobjects;

                let xobject = resolver
                    .get::<XObject>(
                        *xobjects
                            .get(name)
                            .context("Couldn't get XObject while extracting image")?,
                    )
                    .context("context")?;
                // .deref();
                // .get(name)
                match xobject.deref()
                    // .get_inner()
                {
                    XObject::Image(img) => {
                        image::save_buffer(
                            dest,
                            &img.image_data(resolver)?,
                            img.width.try_into()?,
                            img.height.try_into()?,
                            ExtendedColorType::Rgb8,
                        )?;
                        break;
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    }
    // println!("{:#?}", ops);
    Ok(())
}
