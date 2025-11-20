#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn debug_paragraph_events() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertHardBreaks, ConvertSoftBreaks, MergeConsecutiveParagraphs};
    
    let md = "Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
    println!("\n=== Raw Markdown Events ===");
    let raw_events: Vec<_> = MarkdownIter(Parser::new(&md)).collect();
    for (i, event) in raw_events.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    println!("\n=== After ConvertParagraphs ===");
    let after_paragraphs: Vec<_> = ConvertParagraphs::new(MarkdownIter(Parser::new(&md))).collect();
    for (i, event) in after_paragraphs.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    println!("\n=== After All Converters (before MergeConsecutiveParagraphs) ===");
    let before_merge: Vec<_> = ConvertImages::new(
        ConvertText::new(
            ConvertHardBreaks::new(
                ConvertSoftBreaks::new(
                    ConvertParagraphs::new(
                        MarkdownIter(Parser::new(&md))
                    )
                )
            )
        )
    ).collect();
    for (i, event) in before_merge.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    println!("\n=== After MergeConsecutiveParagraphs ===");
    let after_merge: Vec<_> = MergeConsecutiveParagraphs::new(
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
    ).collect();
    for (i, event) in after_merge.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
}

