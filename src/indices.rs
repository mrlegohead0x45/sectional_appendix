// use lopdf::Document;
// use regex::Regex;

// pub(crate) fn extract_indices(
//     doc: &Document,
//     page_range: (u32, u32),
// ) -> Result<Vec<(String, Vec<String>)>, Box<dyn std::error::Error>> {
//     let re = Regex::new(
//         r"^(\w[\w\s\.()]*)\s*(?:(MD\d{3} \n- \n\d{3}) \n- \nLNW\(S\)2)(?:,\s+(MD\d{3} \n- \n\d{3}) \n- \nLNW\(S\)2)*",
//     )?;
//     let hay = doc.extract_text(&(page_range.0..=page_range.1).collect::<Vec<u32>>())?;
//     println!("{}", hay.len());
//     println!("{:?}", &hay[..400]);
//     let mut results = Vec::new();

//     let one_or_more_spaces = Regex::new(" +")?;
//     let any_whitespace = Regex::new(r"[\s,]*")?;

//     for c in re.captures_iter(&hay) {
//         let mut groups = c.iter();
//         groups.next(); // skip over implicit whole match group
//         let name = one_or_more_spaces
//             .replace_all(
//                 &groups
//                     .next()
//                     .ok_or("location name not found")?
//                     .unwrap()
//                     .as_str()
//                     // .to_string()
//                     .trim()
//                     .replace("\n", ""),
//                 " ",
//             )
//             .into_owned();
//         println!("{}", name);
//         let locations: Vec<String> = groups
//             .map(|g| {
//                 any_whitespace
//                     .replace_all(g.unwrap().as_str(), "")
//                     .into_owned()
//             })
//             .collect();
//         results.push((name, locations));
//     }

//     Ok(results)
// }
