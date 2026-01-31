use std::borrow::Cow;

use pdf::{
    content::{Matrix, Op, TextDrawAdjusted},
    file::File,
    object::PageRc,
};

fn text_objects(operations: &[Op]) -> impl Iterator<Item = TextObject> {
    TextObjectParser {
        ops: operations.iter(),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TextObject {
    pub x: f32,
    pub y: f32,
    pub text: String,
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

        while let Some(op) = self.ops.next() {
            match op {
                Op::BeginText => {
                    last_coords = None;
                    last_text = None;
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
                    if let (Some((x, y)), Some(text)) = (last_coords.take(), last_text.take()) {
                        return Some(TextObject { x, y, text });
                    }
                }
                _ => continue,
            }
        }

        None
    }
}
