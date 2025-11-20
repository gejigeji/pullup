#[cfg(all(feature = "markdown", feature = "typst", feature = "builder", feature = "mdbook"))]
#[test]
fn test_paragraph_with_linebreak_using_builder() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::mdbook::to::typst::Conversion;
    use pulldown_typst::markup::TypstMarkup;
    
    // Test case: paragraph with title, two blank lines, then content
    // This should produce a single #par()[] with all content inside
    let md = "Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
    // Use the builder to convert markdown to typst events
    // We need to wrap the markdown iterator in a way that the builder expects
    // The builder expects mdbook::Event, so we need to convert markdown events to mdbook events
    use pullup::mdbook::Event as MdbookEvent;
    let mdbook_events = MarkdownIter(Parser::new(&md)).map(|e| {
        match e {
            pullup::ParserEvent::Markdown(m) => MdbookEvent::MarkdownContentEvent(m),
            _ => panic!("Unexpected event type"),
        }
    });
    
    let events = Conversion::builder()
        .events(mdbook_events)
        .build();
    
    // Collect events
    let typst_events: Vec<_> = events
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te),
                _ => None,
            }
        })
        .collect();
    
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("Actual output:\n{}", output);
    
    // Expected output should be:
    // #par()[Ack消息的重发逻辑：#linebreak()
    //
    // 标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	]
    //
    // NOT:
    // #par()[Ack消息的重发逻辑：#linebreak()
    //
    // #par()[标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	]
    
    // Check that content is NOT wrapped in an extra #par()[]
    assert!(!output.contains("#par()[标记为需要ack的消息发出后"), 
        "Content should not be wrapped in an extra #par()[]");
    
    // Check that the paragraph contains the title
    assert!(output.contains("#par()[Ack消息的重发逻辑："), 
        "Should have paragraph with title");
    
    // Check that the paragraph contains the content (without extra #par())
    let expected_content = "标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复";
    assert!(output.contains(expected_content), 
        "Should contain the expected content");
    
    // Count the number of #par()[ occurrences - should be exactly 1
    let par_count = output.matches("#par()[").count();
    assert_eq!(par_count, 1, 
        "Should have exactly one #par()[], but found {}", par_count);
}

