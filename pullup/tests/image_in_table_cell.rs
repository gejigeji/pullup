#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_image_in_table_cell() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertTables};
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "| Header1 | Header2 |
|---------|---------|
| Cell1   | ![Image](./test.png) |
";
    
    // Convert markdown to typst events (same order as builder.rs)
    // First, let's see what ConvertParagraphs produces
    let events_paragraphs = ConvertParagraphs::new(
        ConvertTables::new(
            MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_TABLES,
            ))
        )
    );
    
    println!("After ConvertParagraphs:");
    let events_paragraphs_vec: Vec<_> = events_paragraphs.collect();
    for (i, event) in events_paragraphs_vec.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    println!();
    
    // Now with ConvertImages
    let events = ConvertImages::new(
        ConvertText::new(
            ConvertParagraphs::new(
                ConvertTables::new(
                    MarkdownIter(Parser::new_ext(
                        &md,
                        pulldown_cmark::Options::ENABLE_TABLES,
                    ))
                )
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
    
    println!("\nAll events after ConvertImages ({}):", all_events.len());
    for (i, event) in all_events.iter().enumerate() {
        println!("{}: {:?}", i, event);
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
    
    println!("Typst events ({}):", typst_events.len());
    for (i, event) in typst_events.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    // Convert to Typst markup - this should not panic
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("Output:\n{}", output);
    
    // Should have table structure
    assert!(output.contains("#table"), "Should have table");
    assert!(output.contains("#image"), "Should have image");
}

