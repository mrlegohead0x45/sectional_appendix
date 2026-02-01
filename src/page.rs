use ord_subset::OrdVar;
use pdf::{
    content::{Matrix, Op, TextDrawAdjusted},
    primitive::{Name, PdfString},
};

pub(crate) fn text_objects(operations: &[Op]) -> impl Iterator<Item = RawTextObject> {
    TextObjectParser {
        ops: operations.iter(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawTextObject {
    pub x: OrdVar<f32>,
    pub y: OrdVar<f32>,
    pub text: Vec<u8>,
    pub font_size: f32,
    pub font_name: Name,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnicodeTextObject {
    pub x: OrdVar<f32>,
    pub y: OrdVar<f32>,
    pub text: String,
    pub font_size: f32,
    pub font_name: Name,
    pub font_avg_width: f32,
}

#[derive(Debug, Clone)]
struct TextObjectParser<'a> {
    ops: std::slice::Iter<'a, Op>,
}

impl Iterator for TextObjectParser<'_> {
    type Item = RawTextObject;

    fn next(&mut self) -> Option<Self::Item> {
        let mut last_coords = None;
        let mut last_text = None;
        let mut last_font_size = None;
        let mut last_font_name = None;

        for op in self.ops.by_ref() {
            match op {
                Op::BeginText => {
                    last_coords = None;
                    last_text = None;
                }
                Op::TextFont { size, name } => {
                    last_font_size = Some(*size);
                    last_font_name = Some(name.clone());
                }
                Op::SetTextMatrix {
                    matrix: Matrix { e, f, .. },
                } => {
                    last_coords = Some((*e, *f));
                }
                Op::TextDraw { text } => last_text = Some(text.data.as_slice().to_vec()),
                Op::TextDrawAdjusted { array } => {
                    last_text = Some(
                        array
                            .iter()
                            .filter_map(|i| match i {
                                TextDrawAdjusted::Spacing(_) => None,
                                TextDrawAdjusted::Text(t) => {
                                    // println!("{:?}", t.data);
                                    // if let Ok(s) = t.to_string() {
                                    //     Some(s)
                                    // } else {
                                    //     println!("INVALID FOUND {:?}", t.data);
                                    //     // println!("{:?}", last_text);
                                    //     // panic!();
                                    //     None
                                    // }
                                    // Some(t.to_string().expect("valid utf8 data"))
                                    Some(t.data.as_slice())
                                }
                            })
                            .flatten()
                            .copied()
                            .collect::<Vec<u8>>(),
                    );
                }
                Op::EndText => {
                    if let (Some((x, y)), Some(text), Some(font_size), Some(font_name)) = (
                        last_coords.take(),
                        last_text.take(),
                        last_font_size.take(),
                        last_font_name.take(),
                    ) {
                        return Some(RawTextObject {
                            x: OrdVar::new(x),
                            y: OrdVar::new(y),
                            text,
                            font_size,
                            font_name,
                        });
                    }
                }
                _ => continue,
            }
        }

        None
    }
}
