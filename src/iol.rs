use anyhow::{Context, Error, Result};
use pdf::{
    PdfError,
    object::{PageRc, Resolve},
};

use crate::strings::extract_grouped_strings;

pub(crate) fn get_iol_strings(
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

pub(crate) fn get_iol_page_no(
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
