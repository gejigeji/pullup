//! Example showing how to use MergeConsecutiveParagraphs to fix paragraph nesting issues
#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn example_how_to_use_merge_consecutive_paragraphs() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::*;
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
    // IMPORTANT: MergeConsecutiveParagraphs must be used AFTER ConvertImages
    // The correct order is:
    // 1. ConvertParagraphs
    // 2. ConvertSoftBreaks
    // 3. ConvertHardBreaks
    // 4. ConvertText
    // 5. ConvertImages (or other converters)
    // 6. MergeConsecutiveParagraphs (MUST be last, after ConvertImages)
    
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
    
    let typst_events: Vec<_> = events
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te),
                _ => None,
            }
        })
        .collect();
    
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    // Expected: Single paragraph with linebreak
    // #par()[Ack消息的重发逻辑：#linebreak()
    // 标记为需要ack的消息发出后...]
    
    // NOT: Two separate paragraphs
    // #par()[Ack消息的重发逻辑：]
    // #par()[标记为需要ack的消息发出后...]
    
    let par_count = output.matches("#par()[").count();
    assert_eq!(par_count, 1, 
        "Should have exactly one paragraph. Got: {}", output);
    assert!(!output.contains("#par()[标记为需要ack的消息发出后"), 
        "Content should not be wrapped in an extra paragraph. Got: {}", output);
}

