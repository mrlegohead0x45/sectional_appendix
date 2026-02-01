use ord_subset::OrdVar;
use pdf::{
    content::{Matrix, Op, TextDrawAdjusted},
    primitive::Name,
};

pub(crate) fn text_objects(operations: &[Op]) -> impl Iterator<Item = TextObject> {
    TextObjectParser {
        ops: operations.iter(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextObject {
    pub x: OrdVar<f32>,
    pub y: OrdVar<f32>,
    pub text: String,
    pub font_size: f32,
    pub font_name: Name,
}

#[derive(Debug, Clone)]
struct TextObjectParser<'a> {
    ops: std::slice::Iter<'a, Op>,
}

impl Iterator for TextObjectParser<'_> {
    type Item = TextObject;

    fn next(&mut self) -> Option<Self::Item> {
        let mut last_coords = None;
        let mut last_text = None;
        let mut last_font_size = None;
        let mut last_font_name = None;

        while let Some(op) = self.ops.next() {
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
                Op::TextDraw { text } => last_text = Some(text.to_string_lossy()),
                Op::TextDrawAdjusted { array } => {
                    last_text = Some(
                        array
                            .iter()
                            .filter_map(|i| match i {
                                TextDrawAdjusted::Spacing(_) => None,
                                TextDrawAdjusted::Text(t) => Some(t.to_string_lossy()),
                            })
                            .collect(),
                    );
                }
                Op::EndText => {
                    if let (Some((x, y)), Some(text), Some(font_size), Some(font_name)) = (
                        last_coords.take(),
                        last_text.take(),
                        last_font_size.take(),
                        last_font_name.take(),
                    ) {
                        return Some(TextObject {
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
