// use regex::Regex;

// struct Urlizer {
//     trailing_punctuation_chars: &'static str,
//     wrapping_punctuation: Vec<(&'static str, &'static str)>,
//     simple_url_re: Regex,
//     simple_url_2_re: Regex,
//     word_split_re: Regex,
//     mailto_template: &'static str,
//     url_template: &'static str,
// }

// impl Urlizer {
//     fn new() -> Urlizer {
//         Urlizer {
//             trailing_punctuation_chars: ".,:;!",
//             wrapping_punctuation: vec![("(", ")"), ("[", "]")],
//             simple_url_re: Regex::new(r"^https?://\[?\w").unwrap(),
//             simple_url_2_re: Regex::new(r"^www\.|^(?!http)\w[^@]+\.(com|edu|gov|int|mil|net|org)($|/.*)$").unwrap(),
//             word_split_re: Regex::new(r"([\s<>]+)").unwrap(),
//             mailto_template: "mailto:{local}@{domain}",
//             url_template: r#"<a href="{href}"{attrs}>{url}</a>"#,
//         }
//     }

//     fn call(
//         &self,
//         text: &str,
//         trim_url_limit: Option<usize>,
//         nofollow: bool,
//         autoescape: bool,
//     ) -> String {
//         let safe_input = text.is_empty();

//         let words: Vec<&str> = self.word_split_re.split(text).collect();
//         words
//             .iter()
//             .map(|&word| {
//                 self.handle_word(
//                     word,
//                     safe_input,
//                     trim_url_limit,
//                     nofollow,
//                     autoescape,
//                 )
//             })
//             .collect()
//     }

//     fn handle_word(
//         &self,
//         word: &str,
//         safe_input: bool,
//         trim_url_limit: Option<usize>,
//         nofollow: bool,
//         autoescape: bool,
//     ) -> String {
//         if word.contains('.') || word.contains('@') || word.contains(':') {
//             let (lead, middle, trail) = self.trim_punctuation(word);

//             let mut url = None;
//             let mut nofollow_attr = if nofollow { " rel=\"nofollow\"" } else { "" };

//             if self.simple_url_re.is_match(middle) {
//                 url = Some(urlencoding::encode(middle).to_string());
//             } else if self.simple_url_2_re.is_match(middle) {
//                 url = Some(urlencoding::encode(format!("http://{}", middle)).to_string());
//             } else if !middle.contains(':') && self.is_email_simple(middle) {
//                 let (local, domain) = middle.rsplitn(2, '@').unwrap();
//                 let domain = punycode::encode_str(domain).unwrap_or_else(|_| return word.to_string());
//                 url = Some(format!(self.mailto_template, local = local, domain = domain));
//                 nofollow_attr = "";
//             }

//             if let Some(url) = url {
//                 let trimmed = self.trim_url(&middle, trim_url_limit);
//                 let (lead, trail) = if autoescape && !safe_input {
//                     (escape(lead), escape(trail))
//                 } else {
//                     (lead.to_string(), trail.to_string())
//                 };

//                 let middle = format!(
//                     self.url_template,
//                     href = escape(&url),
//                     attrs = nofollow_attr,
//                     url = escape(&trimmed),
//                 );

//                 format!("{}{}{}", lead, middle, trail)
//             } else {
//                 if safe_input {
//                     word.to_string()
//                 } else if autoescape {
//                     escape(word)
//                 } else {
//                     word.to_string()
//                 }
//             }
//         } else if safe_input {
//             word.to_string()
//         } else if autoescape {
//             escape(word)
//         } else {
//             word.to_string()
//         }
//     }

//     fn trim_url(&self, x: &str, limit: Option<usize>) -> String {
//         if let Some(limit) = limit {
//             if x.len() <= limit {
//                 return x.to_string();
//             } else {
//                 return format!("{}…", &x[..limit - 1]);
//             }
//         }
//         x.to_string()
//     }

//     fn trim_punctuation(&self, word: &str) -> (String, String, String) {
//         let mut lead = String::new();
//         let mut middle = word.to_string();
//         let mut trail = String::new();

//         let mut trimmed_something = true;
//         while trimmed_something {
//             trimmed_something = false;

//             for (opening, closing) in &self.wrapping_punctuation {
//                 if middle.starts_with(opening) {
//                     middle = middle.trim_start_matches(opening).to_string();
//                     lead += opening;
//                     trimmed_something = true;
//                 }

//                 if middle.ends_with(closing)
//                     && middle.matches(closing).count() == middle.matches(opening).count() + 1
//                 {
//                     middle = middle.trim_end_matches(closing).to_string();
//                     trail = format!("{}{}", closing, &trail);
//                     trimmed_something = true;
//                 }
//             }

//             let middle_unescaped = htmlescape::decode_html(&middle).to_string();
//             let stripped = middle_unescaped.trim_end_matches(&self.trailing_punctuation_chars);
//             if middle_unescaped != stripped {
//                 let punctuation_count = middle_unescaped.len() - stripped.len();
//                 trail = format!("{}{}", &middle[middle.len() - punctuation_count..], &trail);
//                 middle = middle[..middle.len() - punctuation_count].to_string();
//                 trimmed_something = true;
//             }
//         }

//         (lead, middle, trail)
//     }

//     fn is_email_simple(&self, value: &str) -> bool {
//         if !value.contains('@') || value.starts_with('@') || value.ends_with('@') {
//             return false;
//         }

//         if let Some((p1, p2)) = value.rsplitn(2, '@').collect_tuple() {
//             if !p2.contains('.') || p2.starts_with('.') {
//                 return false;
//             }
//             return true;
//         }

//         false
//     }
// }

// fn escape(input: &str) -> String {
//     // Add your custom escape logic here if needed
//     input.to_string()
// }

// // fn main() {
// //     let urlizer = Urlizer::new();
// //     let text = "Your input text here";
// //     let result = urlizer.call(text, Some(50), true, true);
// //     println!("{}", result);
// // }
