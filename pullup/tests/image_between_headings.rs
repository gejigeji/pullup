#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_image_between_headings() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertHeadings};
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "### 系统交互框图  

![系统交互框图](./images/spx/image1.png)

### 规范与约定  
";
    
    // Print raw markdown events first
    println!("Raw Markdown events:");
    for (i, event) in Parser::new(&md).enumerate() {
        println!("{}: {:?}", i, event);
    }
    println!();
    
    // Convert markdown to typst events (same order as builder.rs: Headings -> Paragraphs -> Text -> Images)
    let events = ConvertImages::new(
        ConvertText::new(
            ConvertParagraphs::new(
                ConvertHeadings::new(
                    MarkdownIter(Parser::new(&md))
                )
            )
        )
    );
    
    // Collect all events
    let all_events: Vec<_> = events.collect();
    
    println!("All events ({}):", all_events.len());
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
    
    // Convert to Typst markup
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("Output:\n{}", output);
    
    // Should have first heading
    assert!(output.contains("=== 系统交互框图") || output.contains("==== 系统交互框图"),
        "Should have first heading. Got: {}", output);
    
    // Should have image
    assert!(output.contains("#image(\"images/spx/image1.png\")"),
        "Should have image. Got: {}", output);
    
    // Should have second heading
    assert!(output.contains("=== 规范与约定") || output.contains("==== 规范与约定"),
        "Should have second heading. Got: {}", output);
    
    // Expected output
    let expected_output = "=== 系统交互框图\n#image(\"images/spx/image1.png\")\n=== 规范与约定\n";
    similar_asserts::assert_eq!(output, expected_output);
}

