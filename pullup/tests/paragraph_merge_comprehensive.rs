#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_paragraph_merge_all_scenarios() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::*;
    use pulldown_typst::markup::TypstMarkup;
    
    let md = "Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理	";
    
    // Test scenario 1: Without MergeConsecutiveParagraphs (should fail)
    println!("\n=== Scenario 1: Without MergeConsecutiveParagraphs ===");
    let events1 = ConvertImages::new(
        ConvertText::new(
            ConvertHardBreaks::new(
                ConvertSoftBreaks::new(
                    ConvertParagraphs::new(
                        MarkdownIter(Parser::new(&md))
                    )
                )
            )
        )
    );
    
    let typst_events1: Vec<_> = events1
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te),
                _ => None,
            }
        })
        .collect();
    
    let output1: String = TypstMarkup::new(typst_events1.into_iter()).collect();
    println!("Output: {}", output1);
    let par_count1 = output1.matches("#par()[").count();
    println!("Paragraph count: {}", par_count1);
    assert_eq!(par_count1, 2, "Without MergeConsecutiveParagraphs, should have 2 paragraphs");
    
    // Test scenario 2: With MergeConsecutiveParagraphs (should pass)
    println!("\n=== Scenario 2: With MergeConsecutiveParagraphs ===");
    let events2 = MergeConsecutiveParagraphs::new(
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
    
    let typst_events2: Vec<_> = events2
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te),
                _ => None,
            }
        })
        .collect();
    
    let output2: String = TypstMarkup::new(typst_events2.into_iter()).collect();
    println!("Output: {}", output2);
    let par_count2 = output2.matches("#par()[").count();
    println!("Paragraph count: {}", par_count2);
    assert_eq!(par_count2, 1, "With MergeConsecutiveParagraphs, should have 1 paragraph");
    assert!(!output2.contains("#par()[标记为需要ack的消息发出后"), 
        "Content should not be wrapped in an extra #par()[]");
    
    // Test scenario 3: Different converter order
    println!("\n=== Scenario 3: Different converter order ===");
    let events3 = MergeConsecutiveParagraphs::new(
        ConvertText::new(
            ConvertImages::new(
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
    
    let typst_events3: Vec<_> = events3
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te),
                _ => None,
            }
        })
        .collect();
    
    let output3: String = TypstMarkup::new(typst_events3.into_iter()).collect();
    println!("Output: {}", output3);
    let par_count3 = output3.matches("#par()[").count();
    println!("Paragraph count: {}", par_count3);
    assert_eq!(par_count3, 1, "With MergeConsecutiveParagraphs in different order, should have 1 paragraph");
}

