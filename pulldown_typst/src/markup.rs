use crate::{Event, LinkType, QuoteQuotes, QuoteType, ShowType, TableCellAlignment, Tag};
use std::{collections::VecDeque, fmt::Write, io::ErrorKind};

fn typst_escape(s: &str) -> String {
    s.replace('$', "\\$")
        .replace('#', "\\#")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('*', "\\*")
        .replace('_', " \\_")
        .replace('`', "\\`")
        .replace('@', "\\@")
}

/// Convert Typst events to Typst markup.
///
/// Note: while each item returned by the iterator is a `String`, items may contain
/// multiple lines.
// TODO: tests
pub struct TypstMarkup<'a, T> {
    tag_queue: VecDeque<Tag<'a>>,
    codeblock_queue: VecDeque<()>,
    row_buffer: Option<String>,
    cell_buffer: Option<String>,
    iter: T,
}

impl<'a, T> TypstMarkup<'a, T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    pub fn new(iter: T) -> Self {
        Self {
            tag_queue: VecDeque::new(),
            codeblock_queue: VecDeque::new(),
            row_buffer: None,
            cell_buffer: None,
            iter,
        }
    }
}

impl<'a, T> Iterator for TypstMarkup<'a, T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have a row buffer and it's not empty, we need to handle buffered output first
        // But actually, we'll handle it when the row ends
        
        match self.iter.next() {
            None => {
                // If there's remaining buffer, return it
                if let Some(mut buf) = self.row_buffer.take() {
                    if buf.ends_with(", ") {
                        buf.truncate(buf.len() - 2);
                    }
                    Some(buf)
                } else {
                    None
                }
            }
            Some(Event::Start(x)) => {
                let ret = match x {
                    Tag::Paragraph => Some("#par()[".to_string()),
                    Tag::Show(ty, ref selector, ref set, ref func) => match ty {
                        ShowType::ShowSet => {
                            let (ele, k, v) = set.as_ref().expect("set data for show-set");
                            Some(
                                format!("#show {}: set {}({}:{})", selector, ele, k, v).to_string(),
                            )
                        }
                        ShowType::Function => Some(
                            format!(
                                "#show {}:{}",
                                selector,
                                func.as_ref().expect("function body"),
                            )
                            .to_string(),
                        ),
                    },
                    Tag::Heading(n, _, _) => Some(format!("{} ", "=".repeat(n.get().into()))),
                    // TODO: get the number of backticks / tildes somehow.
                    Tag::CodeBlock(ref fence, ref _display) => {
                        let depth = self.codeblock_queue.len();
                        self.codeblock_queue.push_back(());
                        Some(format!(
                            "{}{}\n",
                            "`".repeat(6 + depth),
                            fence
                                .clone()
                                .map(|x| x.into_string())
                                .unwrap_or_else(|| "".to_string())
                        ))
                    }
                    Tag::BulletList(_, _) => None,
                    Tag::NumberedList(_, _, _) => None,
                    Tag::Item => {
                        let list = self.tag_queue.back().expect("list item contained in list");

                        match list {
                            Tag::BulletList(_, _) => Some("- ".to_string()),
                            Tag::NumberedList(_, _, _) => Some("+ ".to_string()),
                            _ => unreachable!(),
                        }
                    }
                    Tag::Emphasis => Some("#emph[".to_string()),
                    Tag::Strong => Some("#strong[".to_string()),
                    Tag::Link(ref ty, ref url) => match ty {
                        LinkType::Content => Some(format!("#link(\"{url}\")[")),
                        LinkType::Url | LinkType::Autolink => Some(format!("#link(\"{url}\")[")),
                    },
                    Tag::Quote(ref ty, ref quotes, ref attribution) => {
                        let block = match ty {
                            &QuoteType::Block => "block: true,",
                            &QuoteType::Inline => "block: false,",
                        };
                        let quotes = match quotes {
                            &QuoteQuotes::DoNotWrapInDoubleQuotes => "quotes: false,",
                            &QuoteQuotes::WrapInDoubleQuotes => "quotes: true,",
                            &QuoteQuotes::Auto => "quotes: auto,",
                        };
                        match attribution {
                            Some(attribution) => Some(format!(
                                "#quote({} {} attribution: [{}])[",
                                block, quotes, attribution
                            )),
                            None => Some(format!("#quote({} {})[", block, quotes)),
                        }
                    }
                    Tag::Table(ref alignment) => {
                        let num_columns = alignment.len();
                        let alignments: Vec<String> = alignment
                            .iter()
                            .map(|a| match a {
                                TableCellAlignment::Left => "left".to_string(),
                                TableCellAlignment::Center => "center".to_string(),
                                TableCellAlignment::Right => "right".to_string(),
                                TableCellAlignment::None => "start".to_string(),
                            })
                            .collect();
                        
                        // Build the table parameters
                        let mut params = vec![format!("columns: {}", num_columns)];
                        if !alignments.iter().all(|a| a == "start") {
                            params.push(format!("align: ({})", alignments.join(", ")));
                        }
                        
                        Some(format!("#table(\n  {},\n", params.join(", ")))
                    }
                    Tag::TableRow => {
                        self.row_buffer = Some(String::new());
                        Some("".to_string())
                    }
                    Tag::TableHead => {
                        self.row_buffer = Some(String::new());
                        Some("".to_string())
                    }
                    Tag::TableCell => {
                        self.cell_buffer = Some(String::new());
                        Some("".to_string())
                    }
                    _ => todo!(),
                };

                // Set the current tag for later processing and return optional event.
                self.tag_queue.push_back(x);
                if ret.is_none() {
                    return Some("".to_string());
                }
                // If we're in a cell buffer (which means we're in a table cell), accumulate to cell buffer
                if let Some(ref mut cell_buf) = self.cell_buffer {
                    cell_buf.push_str(&ret.as_ref().unwrap());
                    Some("".to_string())
                } else if let Some(ref mut buf) = self.row_buffer {
                    // If we're in a row buffer but not in a cell, accumulate to row buffer
                    buf.push_str(&ret.as_ref().unwrap());
                    Some("".to_string())
                } else {
                    ret
                }
            }
            Some(Event::End(x)) => {
                let ret = match x {
                    Tag::Paragraph => Some("]\n".to_string()),
                    Tag::Heading(_, _, _) => Some("\n".to_string()),
                    Tag::Item => Some("\n".to_string()),
                    Tag::Emphasis => Some("]".to_string()),
                    Tag::Strong => Some("]".to_string()),
                    Tag::BulletList(_, _) => Some("".to_string()),
                    Tag::NumberedList(_, _, _) => Some("".to_string()),
                    Tag::CodeBlock(_, _) => {
                        let _ = self.codeblock_queue.pop_back();
                        let depth = self.codeblock_queue.len();
                        Some(format!("{}\n", "`".repeat(6 + depth)))
                    }
                    Tag::Link(ty, _) => match ty {
                        LinkType::Content => Some("]".to_string()),
                        LinkType::Url | LinkType::Autolink => Some("]".to_string()),
                    },
                    Tag::Show(_, _, _, _) => Some("\n".to_string()),
                    Tag::Quote(quote_type, _, _) => Some(match quote_type {
                        QuoteType::Inline => "]".to_string(),
                        QuoteType::Block => "]\n".to_string(),
                    }),
                    Tag::Table(_) => Some(")\n".to_string()),
                    Tag::TableHead => {
                        if let Some(mut buf) = self.row_buffer.take() {
                            // Remove trailing ", " if present
                            if buf.ends_with(", ") {
                                buf.truncate(buf.len() - 2);
                            }
                            // Output row with cells on same line: [cell1], [cell2], ...
                            Some(format!("  {},\n", buf))
                        } else {
                            Some("\n".to_string())
                        }
                    }
                    Tag::TableRow => {
                        if let Some(mut buf) = self.row_buffer.take() {
                            // Remove trailing ", " if present
                            if buf.ends_with(", ") {
                                buf.truncate(buf.len() - 2);
                            }
                            // Output row with cells on same line: [cell1], [cell2], ...
                            Some(format!("  {},\n", buf))
                        } else {
                            Some("\n".to_string())
                        }
                    }
                    Tag::TableCell => {
                        // Get cell content and decide if it needs quotes
                        if let Some(mut cell_content) = self.cell_buffer.take() {
                            // Replace <br> with \n
                            cell_content = cell_content.replace("<br>", "\\n").replace("<br/>", "\\n").replace("<br />", "\\n");
                            
                            // Escape // to \/\/ for table cells
                            cell_content = cell_content.replace("//", "\\/\\/");
                            
                            // Escape * to \* for table cells, but avoid double-escaping
                            // If content came from Event::Text, * is already escaped as \*
                            // If content came from Event::Raw, * needs to be escaped
                            // We replace * only if it's not already escaped (not preceded by \)
                            let mut result = String::with_capacity(cell_content.len() * 2);
                            let mut chars = cell_content.chars().peekable();
                            while let Some(ch) = chars.next() {
                                if ch == '*' {
                                    // Check if previous char was a backslash
                                    if result.ends_with('\\') {
                                        // Already escaped, keep as is
                                        result.push(ch);
                                    } else {
                                        // Not escaped, escape it
                                        result.push_str("\\*");
                                    }
                                } else {
                                    result.push(ch);
                                }
                            }
                            cell_content = result;
                            
                            // Trim whitespace
                            cell_content = cell_content.trim().to_string();
                            
                            // Check if cell content contains Typst markup (starts with #)
                            // If it's plain text, wrap in quotes; otherwise use as-is
                            let _needs_quotes = !cell_content.trim_start().starts_with('#') 
                                && !cell_content.contains("#emph")
                                && !cell_content.contains("#strong")
                                && !cell_content.contains("#link");
                            
                            // Wrap cell content in square brackets: [content]
                            let formatted_cell = if cell_content.is_empty() {
                                "[]".to_string()
                            } else {
                                format!("[{}]", cell_content)
                            };
                            
                            // Append to row buffer with ", " separator
                            if let Some(ref mut buf) = self.row_buffer {
                                if !buf.is_empty() {
                                    buf.push_str(", ");
                                }
                                buf.push_str(&formatted_cell);
                            } else {
                                // If row_buffer doesn't exist, this is an error state
                                // But we'll still output the cell content
                                eprintln!("Warning: TableCell ended but row_buffer is None");
                            }
                        } else {
                            // Empty cell
                            if let Some(ref mut buf) = self.row_buffer {
                                if !buf.is_empty() {
                                    buf.push_str(", ");
                                }
                                buf.push_str("[]");
                            }
                        }
                        Some("".to_string())
                    }
                    _ => todo!(),
                };

                let in_tag = self.tag_queue.pop_back();

                // Make sure we are in a good state.
                assert_eq!(in_tag, Some(x.clone()));
                
                // If we're in a cell buffer (which means we're in a table cell), accumulate to cell buffer
                if let Some(ref mut cell_buf) = self.cell_buffer {
                    cell_buf.push_str(&ret.as_ref().unwrap_or(&"".to_string()));
                    Some("".to_string())
                } else if self.row_buffer.is_some() && !matches!(&x, Tag::TableRow | Tag::TableHead | Tag::TableCell) {
                    // If we're in a row buffer but not in a cell, accumulate to row buffer
                    if let Some(ref mut buf) = self.row_buffer {
                        buf.push_str(&ret.as_ref().unwrap_or(&"".to_string()));
                        Some("".to_string())
                    } else {
                        ret
                    }
                } else {
                    ret
                }
            }
            Some(Event::Raw(x)) => {
                let content = x.into_string();
                if let Some(ref mut cell_buf) = self.cell_buffer {
                    cell_buf.push_str(&content);
                    Some("".to_string())
                } else if let Some(ref mut buf) = self.row_buffer {
                    buf.push_str(&content);
                    Some("".to_string())
                } else {
                    Some(content)
                }
            }
            Some(Event::Text(x)) => {
                let content = if self.codeblock_queue.is_empty() {
                    typst_escape(&x)
                } else {
                    x.into_string()
                };
                if let Some(ref mut cell_buf) = self.cell_buffer {
                    cell_buf.push_str(&content);
                    Some("".to_string())
                } else if let Some(ref mut buf) = self.row_buffer {
                    buf.push_str(&content);
                    Some("".to_string())
                } else {
                    Some(content)
                }
            }
            Some(Event::Code(x)) => {
                let content = format!(
                    "#raw(\"{}\")",
                    x
                        // "Raw" still needs forward slashes escaped or they will break out of
                        // the tag.
                        .replace('\\', r#"\\"#)
                        // "Raw" still needs quotes escaped or they will prematurely end the tag.
                        .replace('"', r#"\""#)
                );
                if let Some(ref mut buf) = self.row_buffer {
                    buf.push_str(&content);
                    Some("".to_string())
                } else {
                    Some(content)
                }
            }
            Some(Event::Linebreak) => Some("#linebreak()\n".to_string()),
            Some(Event::Parbreak) => Some("#parbreak()\n".to_string()),
            Some(Event::PageBreak) => Some("#pagebreak()\n".to_string()),
            Some(Event::Line(start, end, length, angle, stroke)) => {
                let mut parts = vec![];

                if let Some(start) = start {
                    parts.push(format!("start: ({}, {})", start.0, start.1));
                }
                if let Some(end) = end {
                    parts.push(format!("end: ({}, {})", end.0, end.1));
                }
                if let Some(length) = length {
                    parts.push(format!("length: {}", length));
                }
                if let Some(angle) = angle {
                    parts.push(format!("angle: {}", angle));
                }
                if let Some(stroke) = stroke {
                    parts.push(format!("stroke: {}", stroke));
                }

                Some(format!("#line({})\n", parts.join(", ")))
            }
            Some(Event::Let(lhs, rhs)) => Some(format!("#let {lhs} = {rhs}\n")),
            Some(Event::FunctionCall(v, f, args)) => {
                let args = args.join(", ");
                if let Some(v) = v {
                    Some(format!("#{v}.{f}({args})\n"))
                } else {
                    Some(format!("#{f}({args})\n"))
                }
            }
            Some(Event::DocumentFunctionCall(args)) => {
                let args = args.join(", ");
                Some(format!("#document({args})\n"))
            }
            Some(Event::Set(ele, k, v)) => Some(format!("#set {ele}({k}: {v})\n")),
            Some(Event::DocumentSet(k, v)) => Some(format!("#set document({k}: {v})\n")),
        }
    }
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and push it to a `String`.
pub fn push_markup<'a, T>(s: &mut String, iter: T)
where
    T: Iterator<Item = Event<'a>>,
{
    *s = TypstMarkup::new(iter).collect();
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and write it to a `Write`r.
pub fn write_markup<'a, T, W>(w: &mut W, iter: T) -> std::io::Result<()>
where
    T: Iterator<Item = Event<'a>>,
    W: Write,
{
    for e in TypstMarkup::new(iter) {
        w.write_str(&e)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod emphasis {
        use super::*;

        #[test]
        fn inline() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Text("foo bar baz".into()),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[foo bar baz]";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn containing_underscores() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Text("_whatever_".into()),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[ \\_whatever \\_]";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn nested() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Start(Tag::Strong),
                Event::Text("blah".into()),
                Event::End(Tag::Strong),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[#strong[blah]]";
            assert_eq!(&output, &expected);
        }
    }

    mod escape {
        use super::*;

        #[test]
        fn raw_encodes_code() {
            let input = vec![Event::Code("*foo*".into())];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#raw(\"*foo*\")";
            assert_eq!(&output, &expected);
        }

        #[test]
        // https://github.com/LegNeato/mdbook-typst/issues/3
        fn raw_escapes_forward_slash() {
            let input = vec![Event::Code(r#"\"#.into())];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = r####"#raw("\\")"####;
            assert_eq!(&output, &expected);

            let input = vec![
                Event::Start(Tag::Paragraph),
                Event::Text("before ".into()),
                Event::Code(r#"\"#.into()),
                Event::Text(" after".into()),
                Event::End(Tag::Paragraph),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = r####"#par()[before #raw("\\") after]"####.to_string() + "\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn doesnt_escape_codeblock() {
            let input = vec![
                Event::Start(Tag::CodeBlock(None, crate::CodeBlockDisplay::Block)),
                Event::Text("*blah*".into()),
                Event::End(Tag::CodeBlock(None, crate::CodeBlockDisplay::Block)),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "``````\n*blah*``````\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn escapes_link_content() {
            let input = vec![
                Event::Start(Tag::Link(LinkType::Content, "http://example.com".into())),
                Event::Text("*blah*".into()),
                Event::End(Tag::Link(LinkType::Content, "http://example.com".into())),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#link(\"http://example.com\")[\\*blah\\*]";
            assert_eq!(&output, &expected);
        }
    }

    mod quote {
        use super::*;

        #[test]
        fn single() {
            let input = vec![
                Event::Start(Tag::Quote(QuoteType::Block, QuoteQuotes::Auto, None)),
                Event::Text("to be or not to be".into()),
                Event::End(Tag::Quote(QuoteType::Block, QuoteQuotes::Auto, None)),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#quote(block: true, quotes: auto,)[to be or not to be]\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn attribution() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("to be or not to be".into()),
                Event::End(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected =
                "#quote(block: true, quotes: auto, attribution: [some dude])[to be or not to be]\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn inline_no_newline() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Inline,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("whatever".into()),
                Event::End(Tag::Quote(
                    QuoteType::Inline,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            assert!(!output.contains('\n'));
        }

        #[test]
        fn block_has_newline() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("whatever".into()),
                Event::End(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            assert!(output.contains('\n'));
        }
    }

    mod line {
        use super::*;

        #[test]
        fn basic() {
            let input = vec![Event::Line(None, None, None, None, None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line()\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn start() {
            let input = vec![Event::Line(
                Some(("1".into(), "2".into())),
                None,
                None,
                None,
                None,
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(start: (1, 2))\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn end() {
            let input = vec![Event::Line(
                None,
                Some(("3".into(), "4".into())),
                None,
                None,
                None,
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(end: (3, 4))\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn length() {
            let input = vec![Event::Line(None, None, Some("5".into()), None, None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(length: 5)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn angle() {
            let input = vec![Event::Line(None, None, None, Some("6".into()), None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(angle: 6)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn stroke() {
            let input = vec![Event::Line(None, None, None, None, Some("7".into()))];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(stroke: 7)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn all() {
            let input = vec![Event::Line(
                Some(("1".into(), "2".into())),
                Some(("3".into(), "4".into())),
                Some("5".into()),
                Some("6".into()),
                Some("7".into()),
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(start: (1, 2), end: (3, 4), length: 5, angle: 6, stroke: 7)\n";
            assert_eq!(&output, &expected);
        }
    }

    #[test]
    fn table_conversion() {
        let input = vec![
            Event::Start(Tag::Table(vec![
                TableCellAlignment::Left,
                TableCellAlignment::Center,
            ])),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("Header 1".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("Header 2".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![
                TableCellAlignment::Left,
                TableCellAlignment::Center,
            ])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        let expected =
            "#table(\n  columns: 2, align: (left, center),\n  [Header 1], [Header 2],\n)\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn table_multiple_cells() {
        let input = vec![
            Event::Start(Tag::Table(vec![
                TableCellAlignment::None,
                TableCellAlignment::None,
                TableCellAlignment::None,
            ])),
            Event::Start(Tag::TableHead),
            Event::Start(Tag::TableCell),
            Event::Text("序号".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("版本".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("版本号".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableHead),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("1".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("V1.0".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("1".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![
                TableCellAlignment::None,
                TableCellAlignment::None,
                TableCellAlignment::None,
            ])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        // Each cell should be in separate array elements
        let expected = "#table(\n  columns: 3,\n  [序号], [版本], [版本号],\n  [1], [V1.0], [1],\n)\n";
        assert_eq!(output, expected, "Cells should be properly separated");
    }

    #[test]
    fn table_escapes_forward_slash() {
        let input = vec![
            Event::Start(Tag::Table(vec![TableCellAlignment::None, TableCellAlignment::None])),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("comment // test".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("path/to/file".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![TableCellAlignment::None, TableCellAlignment::None])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        // Only // should be escaped to \/\/, single / should remain unchanged
        let expected = "#table(\n  columns: 2,\n  [comment \\/\\/ test], [path/to/file],\n)\n";
        assert_eq!(output, expected, "Double forward slashes should be escaped in table cells");
    }

    #[test]
    fn table_escapes_asterisk() {
        let input = vec![
            Event::Start(Tag::Table(vec![TableCellAlignment::None, TableCellAlignment::None])),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("bold *text*".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("item*1".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![TableCellAlignment::None, TableCellAlignment::None])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        let expected = "#table(\n  columns: 2,\n  [bold \\*text\\*], [item\\*1],\n)\n";
        assert_eq!(output, expected, "Asterisks should be escaped in table cells");
    }

    #[test]
    fn table_escapes_both_forward_slash_and_asterisk() {
        let input = vec![
            Event::Start(Tag::Table(vec![TableCellAlignment::None])),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("comment // test *bold*".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![TableCellAlignment::None])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        let expected = "#table(\n  columns: 1,\n  [comment \\/\\/ test \\*bold\\*],\n)\n";
        assert_eq!(output, expected, "Both forward slashes and asterisks should be escaped in table cells");
    }
}
