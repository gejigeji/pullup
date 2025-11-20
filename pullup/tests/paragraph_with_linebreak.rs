#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_paragraph_with_linebreak_and_content() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertHardBreaks, ConvertSoftBreaks, MergeConsecutiveParagraphs};
    use pulldown_typst::markup::TypstMarkup;
    
    // Test case: paragraph with title, linebreak, then content
    // This should produce a single #par()[] with all content inside
    let md = "Ack消息的重发逻辑：  



标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
    // Convert markdown to typst events
    // MergeConsecutiveParagraphs should run after ConvertImages to handle all converted events
    let events = MergeConsecutiveParagraphs::new(
        ConvertImages::new(
            ConvertText::new(
                ConvertHardBreaks::new(
                    ConvertSoftBreaks::new(
                        ConvertParagraphs::new(
                            MarkdownIter(Parser::new(&md))
                        )
                    )
                )
            )
        )
    );
    
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
    //
    //
    // 标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	]
    //
    // NOT:
    // #par()[Ack消息的重发逻辑：#linebreak()
    //
    //
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

