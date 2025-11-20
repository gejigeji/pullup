#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_multiple_paragraphs_should_merge() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::*;
    use pulldown_typst::markup::TypstMarkup;
    
    // User's actual scenario - multiple consecutive paragraphs that should be merged
    // This matches the exact format from user's error report
    let md = "返回码定义：
0表示验证成功
其他状态表示失败，连接需要被断开";
    
    // IMPORTANT: MergeConsecutiveParagraphs MUST be used!
    let events = MergeConsecutiveParagraphs::new(
        ConvertLinks::new(
            ConvertImages::new(
                ConvertText::new(
                    ConvertHardBreaks::new(
                        ConvertSoftBreaks::new(
                            ConvertParagraphs::new(
                                ConvertHeadings::new(
                                    MarkdownIter(Parser::new(&md))
                                )
                            )
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
    
    println!("Output:\n{}", output);
    
    // Should have only ONE paragraph
    let par_count = output.matches("#par()[").count();
    println!("Paragraph count: {}", par_count);
    
    // Should NOT have multiple #par()[] blocks
    assert!(!output.contains("#par()[0表示验证成功"), 
        "Should not have separate paragraph for '0表示验证成功'. Output:\n{}", output);
    assert!(!output.contains("#par()[其他状态表示失败"), 
        "Should not have separate paragraph for '其他状态表示失败'. Output:\n{}", output);
    
    // Should have the merged paragraph with linebreaks
    assert!(output.contains("#par()[返回码定义："), 
        "Should have paragraph with '返回码定义：'. Output:\n{}", output);
    
    // Should have exactly ONE paragraph
    assert_eq!(par_count, 1, 
        "Should have exactly ONE paragraph, but found {}. Output:\n{}", par_count, output);
}

