#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_user_scenario_multiple_paragraphs() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::*;
    use pulldown_typst::markup::TypstMarkup;
    
    // User's actual scenario
    let md = "对于未部署 PLC 的小件快手台，Vendor 无法通过 TCP 协议将图片信息推送给 WCS，此时需要通过该接口进行推送

请求路径：/api/ops/sort/upload_pic_with_detail

请求方式：POST

请求参数";
    
    // IMPORTANT: MergeConsecutiveParagraphs MUST be used!
    // The correct converter chain should include MergeConsecutiveParagraphs
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
    
    println!("Output:\n{}", output);
    
    // Should have only ONE paragraph
    let par_count = output.matches("#par()[").count();
    assert_eq!(par_count, 1, 
        "Should have exactly ONE paragraph, but found {}. Output:\n{}", par_count, output);
    
    // Should NOT have multiple #par()[] blocks
    assert!(!output.contains("#par()[请求路径"), 
        "Should not have separate paragraph for '请求路径'. Output:\n{}", output);
    assert!(!output.contains("#par()[请求方式"), 
        "Should not have separate paragraph for '请求方式'. Output:\n{}", output);
    assert!(!output.contains("#par()[请求参数"), 
        "Should not have separate paragraph for '请求参数'. Output:\n{}", output);
    
    // Should have all content in one paragraph with linebreaks
    assert!(output.contains("对于未部署 PLC"), 
        "Should contain first line. Output:\n{}", output);
    assert!(output.contains("请求路径：/api/ops/sort/upload"), 
        "Should contain second line. Output:\n{}", output);
    assert!(output.contains("请求方式：POST"), 
        "Should contain third line. Output:\n{}", output);
    assert!(output.contains("请求参数"), 
        "Should contain fourth line. Output:\n{}", output);
}

