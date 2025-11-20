#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn debug_final_output() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertHardBreaks, ConvertSoftBreaks, MergeConsecutiveParagraphs};
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
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
    
    println!("\n=== Typst Events ===");
    for (i, event) in typst_events.iter().enumerate() {
        println!("{}: {:?}", i, event);
    }
    
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("\n=== Final Output ===");
    println!("{}", output);
    println!("\n=== Output with visible newlines ===");
    for (i, line) in output.lines().enumerate() {
        println!("Line {}: [{}]", i, line);
    }
}

