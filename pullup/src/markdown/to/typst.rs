//! Convert Markdown to Typst.
use std::collections::VecDeque;

use crate::converter;
use crate::markdown;
use crate::typst;
use crate::ParserEvent;

converter!(
    /// Convert Markdown paragraphs to Typst paragraphs.
    ConvertParagraphs,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)))
            },
            x => x,
    }
});

/// Convert Markdown text to Typst text.
pub struct ConvertText<T> {
    code: VecDeque<()>,
    iter: T,
}

impl<'a, T> ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    pub fn new(iter: T) -> Self {
        ConvertText {
            code: VecDeque::new(),
            iter,
        }
    }
}

impl<'a, T> Iterator for ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.code.pop_back(), self.iter.next()) {
            // In code, include the unescaped text.
            (Some(_), Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                Some(ParserEvent::Typst(typst::Event::Text(t)))
            }
            // Not in code, escape the text using typist escaping rules.
            (None, Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                if t.trim().starts_with("\\[") && t.trim().ends_with("\\]") {
                    // Strip out mdbook's non-standard MathJax.
                    // TODO: Translate to typst math and/or expose this as a typed
                    // markdown event.
                    self.next()
                } else {
                    Some(ParserEvent::Typst(typst::Event::Text(t)))
                }
            }
            // Track code start.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::Start(
                    markdown::Tag::CodeBlock(_),
                ))),
            ) => {
                self.code.push_back(());
                event
            }
            // Track code end.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::CodeBlock(
                    _,
                )))),
            ) => {
                let _ = self.code.pop_back();
                event
            }
            (_, x) => x,
        }
    }
}

converter!(
    /// Convert Markdown links to Typst links.
    ConvertLinks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Link(kind, url, _)))) => {
                match kind {
                    markdown::LinkType::Inline => Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Content, url)))),
                    /*
                    markdown::LinkType::Reference => unimplemented!(),
                    markdown::LinkType::ReferenceUnknown => unimplemented!(),
                    markdown::LinkType::Collapsed => unimplemented!(),
                    markdown::LinkType::CollapsedUnknown => unimplemented!(),
                    markdown::LinkType::Shortcut => unimplemented!(),
                    markdown::LinkType::ShortcutUnknown => unimplemented!(),
                    */
                    markdown::LinkType::Autolink => {
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Autolink, url))))
                    },
                    markdown::LinkType::Email => {
                        let url = "mailto:".to_string() + url.as_ref();
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Url, url.into()))))
                    },
                    _ => this.iter.next()
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Link(kind, url, _)))) => {
                match kind {
                    markdown::LinkType::Inline => Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Content, url)))),
                    /*
                    markdown::LinkType::Reference => unimplemented!(),
                    markdown::LinkType::ReferenceUnknown => unimplemented!(),
                    markdown::LinkType::Collapsed => unimplemented!(),
                    markdown::LinkType::CollapsedUnknown => unimplemented!(),
                    markdown::LinkType::Shortcut => unimplemented!(),
                    markdown::LinkType::ShortcutUnknown => unimplemented!(),
                    */
                    markdown::LinkType::Autolink => {
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Autolink, url))))
                    },
                    markdown::LinkType::Email => {
                        let url = "mailto:".to_string() + url.as_ref();
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Url, url.into()))))
                    },
                    _ => this.iter.next()
                }
            },
            x => x,
    }
});

/// Convert Markdown images to Typst image function calls.
/// This converter skips the alt text content inside image tags.
/// It also ensures images are not inside paragraphs by closing the paragraph
/// before the image and reopening it after if needed.
pub struct ConvertImages<'a, T> {
    in_image: bool,
    in_paragraph: bool,
    in_heading: bool,  // Track if we're inside a heading
    paragraph_closed_for_image: bool,  // Track if we closed a paragraph for an image
    buffer: VecDeque<ParserEvent<'a>>,
    iter: T,
}

impl<'a, T> ConvertImages<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    pub fn new(iter: T) -> Self {
        ConvertImages {
            in_image: false,
            in_paragraph: false,
            in_heading: false,
            paragraph_closed_for_image: false,
            buffer: VecDeque::new(),
            iter,
        }
    }
}

impl<'a, T> Iterator for ConvertImages<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have buffered events, return them first
        // But check if it's a heading event to update the flag
        if let Some(event) = self.buffer.pop_front() {
            match &event {
                ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(_, _, _))) => {
                    self.in_heading = true;
                },
                ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(_, _, _))) => {
                    self.in_heading = false;
                },
                _ => {}
            }
            return Some(event);
        }

        match self.iter.next() {
            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(level, toc, bookmarks)))) => {
                // Track that we're entering a heading
                self.in_heading = true;
                // Return the heading start event directly
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(level, toc, bookmarks))))
            },
            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(level, toc, bookmarks)))) => {
                // Track that we're exiting a heading
                self.in_heading = false;
                // Return the heading end event directly
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(level, toc, bookmarks))))
            },
            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) => {
                // If we're inside an image, skip this paragraph (it's the paragraph containing the image)
                if self.in_image {
                    self.next()
                } else {
                    // If we closed a paragraph for an image and now see a new paragraph start,
                    // it means there's content after the image that needs its own paragraph
                    if self.paragraph_closed_for_image {
                        self.paragraph_closed_for_image = false;
                    }
                    // Check if this paragraph contains only a standalone image
                    // Peek at the next event: if it's an image start, skip the paragraph and convert the image
                    match self.iter.next() {
                    Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Image(_, url, _)))) => {
                        // This paragraph contains only an image, skip the paragraph start
                        // and convert the image directly
                        let url_str = url.as_ref().strip_prefix("./").unwrap_or(url.as_ref());
                        let url_str_with_quotes = format!("\"{}\"", url_str);
                        let image_event = ParserEvent::Typst(typst::Event::FunctionCall(None, "image".into(), vec![url_str_with_quotes.into()]));
                        
                        // Skip all content inside image tags (alt text, paragraph tags, etc.)
                        // until we find the image end event
                        loop {
                            match self.iter.next() {
                                Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Image(_, _, _)))) => {
                                    // Found image end, now skip the paragraph end events
                                    loop {
                                        match self.iter.next() {
                                            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) |
                                            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) |
                                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) |
                                            Some(ParserEvent::Typst(typst::Event::Text(_))) |
                                            Some(ParserEvent::Markdown(markdown::Event::Text(_))) => continue,
                                            other => {
                                                // Put back the non-paragraph/text event
                                                if let Some(event) = other {
                                                    self.buffer.push_back(event);
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    break;
                                },
                                Some(_) => continue, // Skip everything inside image tags
                                None => break,
                            }
                        }
                        
                        // Return the image event
                        Some(image_event)
                    },
                        other => {
                            // Not a standalone image, put back the event and return paragraph start
                            if let Some(event) = other {
                                self.buffer.push_back(event);
                            }
                            self.in_paragraph = true;
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                        },
                    }
                }
            },
            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) => {
                self.in_paragraph = false;
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)))
            },
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Image(_, url, _)))) => {
                // Convert image start to FunctionCall event
                // The URL needs to be wrapped in quotes for the function call
                // Remove leading "./" if present
                let url_str = url.as_ref().strip_prefix("./").unwrap_or(url.as_ref());
                let url_str_with_quotes = format!("\"{}\"", url_str);
                let image_event = ParserEvent::Typst(typst::Event::FunctionCall(None, "image".into(), vec![url_str_with_quotes.into()]));
                
                // Set in_image flag to skip alt text
                self.in_image = true;
                
                if self.in_paragraph {
                    // If we're in a paragraph, we need to close it before the image
                    // But we need to check if there's content after the image in the same paragraph
                    // For now, close the paragraph and buffer the image
                    self.buffer.push_back(image_event);
                    self.in_paragraph = false;
                    self.paragraph_closed_for_image = true;
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)))
                } else {
                    Some(image_event)
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Image(_, _, _)))) => {
                self.in_image = false;
                // Images in markdown are wrapped in paragraphs
                // After image end, we need to skip:
                // 1. Any remaining alt text (Markdown or Typst events)
                // 2. Markdown paragraph end
                // 3. Typst paragraph start/end (the paragraph containing the image)
                // Keep skipping until we find something that's not part of the image paragraph
                loop {
                    match self.iter.next() {
                        // Skip alt text (both Markdown and Typst, since ConvertText may have converted it)
                        Some(ParserEvent::Markdown(markdown::Event::Text(_))) => continue,
                        Some(ParserEvent::Typst(typst::Event::Text(_))) => continue,
                        // Skip markdown paragraph end
                        Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) => continue,
                        // Skip typst paragraph tags (the paragraph containing the image)
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) => continue,
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) => {
                            // All wrapped paragraph tags skipped, get next event
                            break self.next();
                        },
                        // Found something else (like a heading), return it
                        other => break other,
                    }
                }
            },
            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) if !self.in_paragraph && self.paragraph_closed_for_image => {
                // This paragraph end was for the paragraph that contained the image
                // We already closed it, so skip this one
                self.paragraph_closed_for_image = false;
                None
            },
            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) if !self.in_paragraph && !self.paragraph_closed_for_image => {
                        // This is an empty paragraph that was created by ConvertParagraphs for a standalone image
                        // Check if there's content after it (could be after soft break)
                        // We need to peek ahead to see if there's content
                        let mut peek_events = Vec::new();
                        let mut found_content = false;
                        let mut next_paragraph_start = false;
                        
                        // Peek at next events to see if there's content
                        loop {
                            match self.iter.next() {
                                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) => {
                                    next_paragraph_start = true;
                                    peek_events.push(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)));
                                    break;
                                },
                                Some(break_event @ ParserEvent::Markdown(markdown::Event::SoftBreak)) | Some(break_event @ ParserEvent::Markdown(markdown::Event::HardBreak)) => {
                                    peek_events.push(break_event);
                                    // Continue to check next event after break
                                    continue;
                                },
                                Some(next_event @ (ParserEvent::Typst(typst::Event::Text(_)) | ParserEvent::Markdown(markdown::Event::Text(_)))) => {
                                    found_content = true;
                                    peek_events.push(next_event);
                                    // Continue to check if there's a paragraph end after the text
                                    match self.iter.next() {
                                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) => {
                                            peek_events.push(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)));
                                        },
                                        other => {
                                            if let Some(event) = other {
                                                peek_events.push(event);
                                            }
                                        },
                                    }
                                    break;
                                },
                                Some(event @ ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(_, _, _)))) => {
                                    // Found a heading start, collect the entire heading
                                    peek_events.push(event);
                                    // Continue to collect heading content and end
                                    loop {
                                        match self.iter.next() {
                                            Some(e @ ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(_, _, _)))) => {
                                                peek_events.push(e);
                                                break;
                                            },
                                            Some(e) => {
                                                peek_events.push(e);
                                                continue;
                                            },
                                            None => break,
                                        }
                                    }
                                    break;
                                },
                                other => {
                                    // Any other event should be preserved
                                    if let Some(event) = other {
                                        peek_events.push(event);
                                    }
                                    break;
                                },
                            }
                        }
                        
                        // Check if there are other events (like headings) before putting them back
                        let has_other_events = !peek_events.is_empty() && !next_paragraph_start && !found_content;
                        
                        // Put back all peeked events in reverse order
                        for event in peek_events.into_iter().rev() {
                            self.buffer.push_front(event);
                        }
                        
                        if next_paragraph_start {
                            // There's a new paragraph after the image
                            self.in_paragraph = true;
                            None  // Skip the empty paragraph end
                        } else if found_content {
                            // There's content after the image, create a new paragraph for it
                            self.in_paragraph = true;
                            // Return paragraph start immediately, content is already in buffer
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                        } else if has_other_events {
                            // There are other events (like headings) after the image
                            // Just skip the empty paragraph end, events are already in buffer
                            None
                        } else {
                            // No content after, just skip the empty paragraph
                            None
                        }
            },
            Some(ParserEvent::Markdown(markdown::Event::SoftBreak)) | Some(ParserEvent::Markdown(markdown::Event::HardBreak)) if !self.in_paragraph && !self.paragraph_closed_for_image => {
                        // After a standalone image, if we see a break followed by content, create a paragraph
                        match self.iter.next() {
                            Some(next_event @ (ParserEvent::Typst(typst::Event::Text(_)) | ParserEvent::Markdown(markdown::Event::Text(_)))) => {
                                self.in_paragraph = true;
                                self.buffer.push_back(next_event);
                                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                            },
                            other => {
                                // Put back the break and return other event
                                self.buffer.push_back(self.iter.next().unwrap());
                                other
                            },
                        }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) => {
                        // This is a markdown paragraph end that comes after the image
                        // If we closed the paragraph before the image, we should skip this one too
                        // Continue to check for the Typst paragraph end
                        match self.iter.next() {
                            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) if !self.in_paragraph && self.paragraph_closed_for_image => {
                                // Skip both the markdown and typst paragraph ends
                                self.paragraph_closed_for_image = false;
                                None
                            },
                            Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) if !self.in_paragraph && !self.paragraph_closed_for_image => {
                                // This is an empty paragraph that was created by ConvertParagraphs for a standalone image
                                // Check if there's content after it (like headings)
                                // Use the same peek logic as the main branch
                                let mut peek_events = Vec::new();
                                let mut found_content = false;
                                let mut next_paragraph_start = false;
                                
                                // Peek at next events to see if there's content
                                loop {
                                    match self.iter.next() {
                                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) => {
                                            next_paragraph_start = true;
                                            peek_events.push(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)));
                                            break;
                                        },
                                        Some(break_event @ ParserEvent::Markdown(markdown::Event::SoftBreak)) | Some(break_event @ ParserEvent::Markdown(markdown::Event::HardBreak)) => {
                                            peek_events.push(break_event);
                                            continue;
                                        },
                                        Some(next_event @ (ParserEvent::Typst(typst::Event::Text(_)) | ParserEvent::Markdown(markdown::Event::Text(_)))) => {
                                            found_content = true;
                                            peek_events.push(next_event);
                                            match self.iter.next() {
                                                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) => {
                                                    peek_events.push(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)));
                                                },
                                                other => {
                                                    if let Some(event) = other {
                                                        peek_events.push(event);
                                                    }
                                                },
                                            }
                                            break;
                                        },
                                        Some(event @ ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(_, _, _)))) => {
                                            peek_events.push(event);
                                            loop {
                                                match self.iter.next() {
                                                    Some(e @ ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(_, _, _)))) => {
                                                        peek_events.push(e);
                                                        break;
                                                    },
                                                    Some(e) => {
                                                        peek_events.push(e);
                                                        continue;
                                                    },
                                                    None => break,
                                                }
                                            }
                                            break;
                                        },
                                        other => {
                                            if let Some(event) = other {
                                                peek_events.push(event);
                                            }
                                            break;
                                        },
                                    }
                                }
                                
                                let has_other_events = !peek_events.is_empty() && !next_paragraph_start && !found_content;
                                
                                // Put back all peeked events in reverse order
                                for event in peek_events.into_iter().rev() {
                                    self.buffer.push_front(event);
                                }
                                
                                if next_paragraph_start {
                                    self.in_paragraph = true;
                                    None
                                } else if found_content {
                                    self.in_paragraph = true;
                                    None
                                } else if has_other_events {
                                    None
                                } else {
                                    None
                                }
                            },
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph))) => {
                                // There's a new paragraph after the image, which is correct
                                self.paragraph_closed_for_image = false;
                                self.in_paragraph = true;
                                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                            },
                            Some(next_event @ (ParserEvent::Typst(typst::Event::Text(_)) | ParserEvent::Markdown(markdown::Event::Text(_)))) => {
                                // There's text content after the image in the same paragraph
                                // We need to create a new paragraph for it
                                self.paragraph_closed_for_image = false;
                                self.in_paragraph = true;
                                self.buffer.push_back(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph)));
                                self.buffer.push_back(next_event);
                                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                            },
                            other => {
                                // Put back the markdown paragraph end and return the other event
                                self.buffer.push_back(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph)));
                                other
                            }
                        }
            },
            Some(event) if self.in_image => {
                // Skip all content inside image tags (alt text and paragraph tags)
                // This includes both Markdown and Typst events (since text may have been converted)
                match event {
                    ParserEvent::Markdown(markdown::Event::Text(_)) => {
                        // Skip markdown text (alt text)
                        self.next()
                    },
                    ParserEvent::Typst(typst::Event::Text(_)) => {
                        // Skip typst text (alt text that was already converted)
                        self.next()
                    },
                    ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)) => {
                        // Skip paragraph start inside image tags
                        self.next()
                    },
                    ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)) => {
                        // Skip paragraph end inside image tags
                        self.next()
                    },
                    ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Image(_, _, _))) => {
                        // Image end - handle it normally
                        self.in_image = false;
                        // Images in markdown are wrapped in paragraphs
                        // Skip the markdown paragraph end and typst paragraph end
                        match self.iter.next() {
                            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) => {
                                match self.iter.next() {
                                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph))) => {
                                        // All wrapped paragraph tags skipped, get next event
                                        self.next()
                                    },
                                    other => other,
                                }
                            },
                            other => other,
                        }
                    },
                    _ => {
                        // Skip any other events inside image tags
                        self.next()
                    },
                }
            },
            Some(next_event @ (ParserEvent::Typst(typst::Event::Text(_)) | ParserEvent::Markdown(markdown::Event::Text(_)))) => {
                        // There's text content after the image
                        // If we're in a heading, just return the text without creating a paragraph
                        if self.in_heading {
                            Some(next_event)
                        } else {
                            // We need to create a new paragraph for it
                            self.paragraph_closed_for_image = false;
                            self.in_paragraph = true;
                            self.buffer.push_back(next_event);
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
                        }
            },
            x => x,
        }
    }
}

converter!(
    /// Convert Markdown **strong** tags to Typst strong tags.
    ConvertStrong,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Strong)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Strong)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown _emphasis_ tags to Typst emphasis tags.
    ConvertEmphasis,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Emphasis)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Emphasis)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown soft breaks to Typst line breaks.
    ConvertSoftBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::SoftBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Text(" ".into())))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown hard breaks to Typst line breaks.
    ConvertHardBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::HardBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Linebreak))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown blockquotes to Typst quotes.
    ConvertBlockQuotes,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::BlockQuote))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::BlockQuote))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown code tags to Typst raw tags.
    ConvertCode,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Inline.
            Some(ParserEvent::Markdown(markdown::Event::Code(x))) => {
                Some(ParserEvent::Typst(typst::Event::Code(x)))
            },
            // Block.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::CodeBlock(kind)))) => {
                match kind {
                    markdown::CodeBlockKind::Indented => Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block)))),
                    markdown::CodeBlockKind::Fenced(val) => {
                        let val = if val.as_ref() == "" {
                            None
                        } else {
                            Some(val)
                        };
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(val, typst::CodeBlockDisplay::Block))))
                    },
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::CodeBlock(kind)))) => {
                match kind {
                    markdown::CodeBlockKind::Indented => Some(ParserEvent::Typst(typst::Event::End(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block)))),
                    markdown::CodeBlockKind::Fenced(val) => {
                        let val = if val.as_ref() == "" {
                            None
                        } else {
                            Some(val)
                        };
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::CodeBlock(val, typst::CodeBlockDisplay::Block))))
                    },
                }
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown lists to Typst lists.
    ConvertLists,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        // TODO: Handle tight.

        // TODO: Allow changing the marker and number format.
        match this.iter.next() {
            // List start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::List(number)))) => {
                if let Some(start) = number {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::NumberedList(start, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::BulletList(None, false))))
                }

            },
            // List end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::List(number)))) => {
                if let Some(start) = number {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::NumberedList(start, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::BulletList(None, false))))
                }

            },
            // List item start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Item))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Item)))
            },
            // List item end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Item))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Item)))
            },
            x => x,
        }
   }
);

converter!(
    /// Convert Markdown headings to Typst headings.
    ConvertHeadings,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        struct TypstLevel(std::num::NonZeroU8);

        impl std::ops::Deref for TypstLevel {
            type Target = std::num::NonZeroU8;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl From<markdown::HeadingLevel> for TypstLevel{
            fn from(item: markdown::HeadingLevel) -> Self {
                use markdown::HeadingLevel;
                match item {
                    HeadingLevel::H1 => TypstLevel(core::num::NonZeroU8::new(1).expect("non-zero")),
                    HeadingLevel::H2 => TypstLevel(core::num::NonZeroU8::new(2).expect("non-zero")),
                    HeadingLevel::H3 => TypstLevel(core::num::NonZeroU8::new(3).expect("non-zero")),
                    HeadingLevel::H4 => TypstLevel(core::num::NonZeroU8::new(4).expect("non-zero")),
                    HeadingLevel::H5 => TypstLevel(core::num::NonZeroU8::new(5).expect("non-zero")),
                    HeadingLevel::H6 => TypstLevel(core::num::NonZeroU8::new(6).expect("non-zero")),
                }
            }
        }
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Heading(level, _, _)))) => {
                let level: TypstLevel = level.into();
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(*level,
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                ))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Heading(level, _, _))))  => {
                let level: TypstLevel = level.into();
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(*level,
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                ))))
            },
            x => x,
        }
   }
);

converter!(
    /// Convert Markdown tables to Typst tables.
    ConvertTables,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Handle starting a table
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Table(alignment)))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Table(
                    alignment.iter().map(|&a| match a {
                        markdown::Alignment::Left => typst::TableCellAlignment::Left,
                        markdown::Alignment::Center => typst::TableCellAlignment::Center,
                        markdown::Alignment::Right => typst::TableCellAlignment::Right,
                        markdown::Alignment::None => typst::TableCellAlignment::None,
                    }).collect(),
                ))))
            },
            // Handle ending a table
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Table(alignment)))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Table(
                    alignment.iter().map(|&a| match a {
                        markdown::Alignment::Left => typst::TableCellAlignment::Left,
                        markdown::Alignment::Center => typst::TableCellAlignment::Center,
                        markdown::Alignment::Right => typst::TableCellAlignment::Right,
                        markdown::Alignment::None => typst::TableCellAlignment::None,
                    }).collect(),
                ))))
            },
            // Handle header row
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableHead))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableHead)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::TableHead))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableHead)))
            },
            // Handle starting a row
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableRow))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableRow)))
            },
            // Handle ending a row
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::TableRow))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableRow)))
            },
            // Handle starting a cell
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableCell))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableCell)))
            },
            // Handle ending a cell
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::TableCell))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableCell)))
            },
            // Pass through any other events
            x => x,
        }
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::CowStr;
    use crate::markdown::{MarkdownIter, Parser};
    use similar_asserts::assert_eq;
    use std::num::NonZeroU8;

    // Set up type names so they are clearer and more succint.
    use markdown::Event as MdEvent;
    use markdown::HeadingLevel;
    use markdown::Tag as MdTag;
    use typst::Event as TypstEvent;
    use typst::Tag as TypstTag;
    use ParserEvent::*;

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#atx-headings
    /// * https://spec.commonmark.org/0.30/#setext-headings Typst docs:
    /// * https://typst.app/docs/reference/meta/heading/
    mod headings {
        use super::*;

        #[test]
        fn convert_headings() {
            let md = "\
# Greetings

## This is **rad**!
";
            let i = ConvertHeadings::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Greetings"))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("This is "))),
                    Markdown(MdEvent::Start(MdTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("rad"))),
                    Markdown(MdEvent::End(MdTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("!"))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#link-reference-definitions
    /// * https://spec.commonmark.org/0.30/#links
    /// * https://spec.commonmark.org/0.30/#autolinks Typst docs:
    /// * https://typst.app/docs/reference/meta/link/
    mod links {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
Cool [beans](https://example.com)
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("beans"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn auto() {
            let md = "\
Cool <https://example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Autolink,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("https://example.com"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Autolink,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn email() {
            let md = "\
Who are <you@example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Who are "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Url,
                        CowStr::Boxed("mailto:you@example.com".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("you@example.com"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Url,
                        CowStr::Boxed("mailto:you@example.com".into())
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/strong/
    mod strong {
        use super::*;
        #[test]
        fn convert_strong() {
            let md = "\
## **Foo**

I **love** cake!
";
            let i = ConvertStrong::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading(
                        HeadingLevel::H2,
                        None,
                        vec![]
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::End(MdTag::Heading(HeadingLevel::H2, None, vec![]))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/emph/
    mod emphasis {
        use super::*;
        #[test]
        fn convert_emphasis() {
            let md = "\
## _Foo_

I *love* cake!
";
            let i = ConvertEmphasis::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading(
                        HeadingLevel::H2,
                        None,
                        vec![]
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::End(MdTag::Heading(HeadingLevel::H2, None, vec![]))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#code Typst docs:
    /// * https://typst.app/docs/reference/text/raw/
    mod code {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
foo `bar` baz
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo "))),
                    Typst(TypstEvent::Code(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" baz"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn block_indent() {
            let md = "\
whatever

    code 1
    code 2
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("whatever"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 1\n"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 2\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block() {
            let md = "\
```
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block_with_fence() {
            let md = "\
```foo
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        Some(CowStr::Borrowed("foo")),
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        Some(CowStr::Borrowed("foo")),
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#text Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod text {
        use super::*;
        #[test]
        fn convert_text() {
            let md = "\
foo

bar

baz
";
            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("foo"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("baz"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.31.2/#hard-line-breaks
    /// * https://spec.commonmark.org/0.31.2/#soft-line-breaks
    /// Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod breaks {
        use super::*;

        #[test]
        fn soft() {
            // Note that "foo" DOES NOT HAVE two spaces after it.
            let md = "\
foo
bar
";
            let i = ConvertSoftBreaks::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Text(CowStr::Borrowed(" "))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn hard() {
            // Note that "foo" has two spaces after it.
            let md = "\
foo  
bar
";
            let i = ConvertHardBreaks::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Linebreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#paragraphs Typst docs:
    /// * https://typst.app/docs/reference/layout/par/
    mod paragraphs {
        use super::*;
        #[test]
        fn convert_paragraphs() {
            let md = "\
foo

bar

baz
";
            let i = ConvertParagraphs::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("baz"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#lists Typst docs:
    /// * https://typst.app/docs/reference/layout/list
    /// * https://typst.app/docs/reference/layout/enum/
    mod lists {
        use super::*;

        #[test]
        fn bullet() {
            let md = "\
* dogs
* are
* cool
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bulet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("dogs"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("are"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Third bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cool"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ],
            );
        }

        #[test]
        fn numbered() {
            let md = "\
1. cats are _too_
2. birds are ok
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(1, None, false))),
                    // First bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cats are "))),
                    Markdown(MdEvent::Start(MdTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("too"))),
                    Markdown(MdEvent::End(MdTag::Emphasis)),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("birds are ok"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::NumberedList(1, None, false))),
                ],
            );
        }

        #[test]
        fn numbered_custom_start() {
            let md = "\
6. foo
1. bar
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(6, None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::NumberedList(6, None, false))),
                ],
            );
        }

        #[test]
        fn multiple_lines() {
            let md = "\
* multiple
  lines
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("multiple"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("lines"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ]
            );
        }
    }

    mod issues {
        use super::*;

        // https://github.com/LegNeato/mdbook-typst/issues/3
        #[test]
        fn backslashes_in_backticks() {
            let md = r###"before `\` after"###;

            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text("before ".into())),
                    Markdown(MdEvent::Code(r#"\"#.into())),
                    Typst(TypstEvent::Text(" after".into())),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn simple_blockquote() {
            let md = "> test";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("test"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn complex_blockquote() {
            let md = "> one\n> two\n> three";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("one"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("two"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("three"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }
    }

    mod images {
        use super::*;

        #[test]
        fn convert_image() {
            let md = "\
整体交互流程图

![整体交互流程图](./images/infeed/image2.png)
";
            let i = ConvertImages::new(MarkdownIter(Parser::new(&md)));
            let events: Vec<_> = i.collect();

            // Check that image function call is present
            let image_call = events.iter().find(|e| {
                matches!(e, Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image")
            });
            assert!(image_call.is_some(), "Should find image function call");
            
            // Check that alt text is skipped
            let has_alt_text = events.iter().any(|e| {
                matches!(e, Markdown(MdEvent::Text(t)) if t.as_ref() == "整体交互流程图" && 
                    events.iter().position(|x| x == e).unwrap() > events.iter().position(|x| matches!(x, Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image")).unwrap())
            });
            assert!(!has_alt_text, "Alt text after image should be skipped");
        }

        #[test]
        fn convert_image_without_prefix() {
            let md = "![alt text](images/test.png)";
            let i = ConvertImages::new(MarkdownIter(Parser::new(&md)));

            let events: Vec<_> = i.collect();
            // Find the FunctionCall event
            let image_call = events.iter().find(|e| {
                matches!(e, Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image")
            });
            assert!(image_call.is_some(), "Should find image function call");
            if let Some(Typst(TypstEvent::FunctionCall(_, _, args))) = image_call {
                similar_asserts::assert_eq!(args[0].as_ref(), "\"images/test.png\"");
            }
        }

        #[test]
        fn convert_image_skips_alt_text() {
            let md = "![This is alt text](image.png)";
            let i = ConvertImages::new(MarkdownIter(Parser::new(&md)));

            let events: Vec<_> = i.collect();
            // Alt text should not appear in the output
            let has_alt_text = events.iter().any(|e| {
                matches!(e, Markdown(MdEvent::Text(t)) if t.as_ref() == "This is alt text")
            });
            assert!(!has_alt_text, "Alt text should be skipped");
            
            // But image call should be present
            let image_call = events.iter().find(|e| {
                matches!(e, Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image")
            });
            assert!(image_call.is_some(), "Should find image function call");
        }

        #[test]
        fn convert_image_in_paragraph_closes_paragraph() {
            // Test that when an image is inside a paragraph, the paragraph is closed before the image
            let md = "Some text ![alt text](image.png) more text";
            let i = ConvertImages::new(ConvertParagraphs::new(MarkdownIter(Parser::new(&md))));

            let events: Vec<_> = i.collect();
            // The paragraph should be closed before the image
            let mut _found_paragraph_end_before_image = false;
            let mut found_image = false;
            for event in &events {
                if matches!(event, Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image") {
                    found_image = true;
                    break;
                }
                if matches!(event, Typst(TypstEvent::End(TypstTag::Paragraph))) {
                    _found_paragraph_end_before_image = true;
                }
            }
            assert!(found_image, "Should find image function call");
            // When image is in paragraph, paragraph should be closed before image
            // Note: This test may need adjustment based on actual markdown parsing behavior
        }

        #[test]
        fn convert_image_after_text_in_same_paragraph() {
            // Test case: text and image in the same paragraph (no blank line between them)
            // This is the case reported by the user
            // Input: "整体交互流程图\n![整体交互流程图](./images/infeed/image2.png)  "
            // Expected: paragraph with text, paragraph end, then image function call
            let md = "\
整体交互流程图
![整体交互流程图](./images/infeed/image2.png)  
";
            // First convert paragraphs, then images, then text
            let i = ConvertText::new(ConvertImages::new(ConvertParagraphs::new(MarkdownIter(Parser::new(&md)))));

            let events: Vec<_> = i.collect();
            
            // Expected sequence: paragraph start, text, paragraph end, image function call
            // The paragraph should be closed before the image
            let mut found_paragraph_start = false;
            let mut found_text = false;
            let mut found_paragraph_end_before_image = false;
            let mut found_image = false;
            
            for event in &events {
                match event {
                    Typst(TypstEvent::Start(TypstTag::Paragraph)) if !found_paragraph_start => {
                        found_paragraph_start = true;
                    },
                    Typst(TypstEvent::Text(_)) if found_paragraph_start && !found_text => {
                        found_text = true;
                    },
                    Typst(TypstEvent::End(TypstTag::Paragraph)) if found_text && !found_image => {
                        found_paragraph_end_before_image = true;
                    },
                    Typst(TypstEvent::FunctionCall(_, f, _)) if f.as_ref() == "image" => {
                        found_image = true;
                    },
                    _ => {},
                }
            }
            
            assert!(found_paragraph_start, "Should find paragraph start");
            assert!(found_text, "Should find text");
            assert!(found_paragraph_end_before_image, "Paragraph should be closed before image");
            assert!(found_image, "Should find image function call");
        }
    }

    mod tables {
        use super::*;

        #[test]
        fn simple_table() {
            let md = "\
| Header1 | Header2 |
|---------|---------|
| Cell1   | Cell2   |
";
            let i = ConvertTables::new(MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_TABLES,
            )));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Table(vec![
                        typst::TableCellAlignment::None,
                        typst::TableCellAlignment::None,
                    ]))),
                    Typst(TypstEvent::Start(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableRow)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableRow)),
                    Typst(TypstEvent::End(TypstTag::Table(vec![
                        typst::TableCellAlignment::None,
                        typst::TableCellAlignment::None,
                    ]))),
                ]
            );
        }

        #[test]
        fn table_with_alignment() {
            let md = "\
| Header1 | Header2 |
|:--------|:-------:|
| Cell1   | Cell2   |
";
            let i = ConvertTables::new(MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_TABLES,
            )));

            similar_asserts::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Table(vec![
                        typst::TableCellAlignment::Left,
                        typst::TableCellAlignment::Center,
                    ]))),
                    Typst(TypstEvent::Start(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableRow)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableRow)),
                    Typst(TypstEvent::End(TypstTag::Table(vec![
                        typst::TableCellAlignment::Left,
                        typst::TableCellAlignment::Center,
                    ]))),
                ]
            );
        }
    }
}
