use anyhow::{Context, Result};
use pdf::object::{PageRc, Resolve};

use crate::{
    text_object::{RawTextObject, UnicodeTextObject, text_objects},
    unicode::unicodify,
};

/// Attempt to group adjacent textobjects forming a single text block into one
pub(crate) fn group_textobjects(objs: Vec<UnicodeTextObject>) -> Vec<UnicodeTextObject> {
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

pub(crate) fn extract_grouped_strings(
    page: PageRc,
    resolver: &impl Resolve,
) -> Result<Vec<String>> {
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
            // dbg!(&o);
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
