#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_standalone_image_no_empty_paragraph() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText};
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "![image](./images/spx/image1.png)";
    
    // Convert markdown to typst events
    let events = ConvertText::new(
        ConvertImages::new(
            ConvertParagraphs::new(
                MarkdownIter(Parser::new(&md))
            )
        )
    );
    
    // Collect all events one by one to see the flow
    let mut all_events = Vec::new();
    let mut event_iter = events;
    loop {
        match event_iter.next() {
            Some(event) => {
                println!("Event {}: {:?}", all_events.len(), event);
                all_events.push(event);
            },
            None => break,
        }
    }
    
    // Collect events and filter to only Typst events
    let typst_events: Vec<_> = all_events
        .iter()
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te.clone()),
                _ => None,
            }
        })
        .collect();
    
    println!("Typst events:");
    for (i, event) in typst_events.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    // Convert to Typst markup
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("Output: {}", output);
    
    // Should NOT have empty paragraph
    assert!(!output.contains("#par()[]"), 
        "Should not have empty paragraph. Got: {}", output);
    
    // Should have image
    assert!(output.contains("#image(\"images/spx/image1.png\")"),
        "Should have image. Got: {}", output);
    
    // Should be just the image, no paragraph
    let expected = "#image(\"images/spx/image1.png\")\n";
    assert_eq!(output, expected, "Output should be just image");
}

