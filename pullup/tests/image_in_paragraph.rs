#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_image_after_text_in_same_paragraph() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText};
    
    let md = "\
整体交互流程图
![整体交互流程图](./images/infeed/image2.png)  
";
    
    // Convert markdown to typst events
    let events = ConvertText::new(
        ConvertImages::new(
            ConvertParagraphs::new(
                MarkdownIter(Parser::new(&md))
            )
        )
    );
    
    // Collect events
    let all_events: Vec<_> = events.collect();
    
    // Check event sequence: paragraph start, text, paragraph end, image function call
    let mut found_paragraph_start = false;
    let mut found_text = false;
    let mut found_paragraph_end_before_image = false;
    let mut found_image = false;
    
    for event in &all_events {
        match event {
            pullup::ParserEvent::Typst(pullup::typst::Event::Start(pullup::typst::Tag::Paragraph)) if !found_paragraph_start => {
                found_paragraph_start = true;
            },
            pullup::ParserEvent::Typst(pullup::typst::Event::Text(_)) if found_paragraph_start && !found_text => {
                found_text = true;
            },
            pullup::ParserEvent::Typst(pullup::typst::Event::End(pullup::typst::Tag::Paragraph)) if found_text && !found_image => {
                found_paragraph_end_before_image = true;
            },
            pullup::ParserEvent::Typst(pullup::typst::Event::FunctionCall(_, f, _)) if f.as_ref() == "image" => {
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

