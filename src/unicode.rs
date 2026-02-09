use std::{collections::HashMap, iter, ops::Deref};

use adobe_cmap_parser::{get_byte_mapping, get_unicode_map};
use anyhow::{Context, Error, Ok, Result, bail};
use pdf::{
    font::{Font, FontData, FontDescriptor, TFont},
    object::{Lazy, Resolve},
    primitive::Name,
};

use crate::text_object::{RawTextObject, UnicodeTextObject};

pub(crate) fn unicodify(
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
        None => Ok(String::from_iter(raw.text.iter().map(|x| char::from(*x))))
            .with_context(|| format!("Invalid UTF8 when no ToUnicode was found"))
            .with_context(|| format!("Font was {:?}", font)),

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
                                // dbg!(&cid);
                                // println!("{}", String::from_utf8(cmap_bytes.to_vec())?);
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
