#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn debug_multiple_paragraphs_with_blanks() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::{ConvertImages, ConvertParagraphs, ConvertText, ConvertHardBreaks, ConvertSoftBreaks, MergeConsecutiveParagraphs};
    use pulldown_typst::markup::TypstMarkup;
    
    // Simulate the user's case: multiple paragraphs with blank lines between them
    let md = "对于未部署 PLC 的小件快手台，Vendor 无法通过 TCP 协议将图片信息推送给 WCS，此时需要通过该接口进行推送

请求路径：/api/ops/sort/upload_pic_with_detail

请求方式：POST

请求参数";
    
    println!("\n=== Raw Markdown Events ===");
    let raw_events: Vec<_> = MarkdownIter(Parser::new(&md)).collect();
    for (i, event) in raw_events.iter().enumerate() {
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
    
    let typst_events: Vec<_> = after_merge
        .iter()
        .filter_map(|e| {
            match e {
                pullup::ParserEvent::Typst(te) => Some(te.clone()),
                _ => None,
            }
        })
        .collect();
    
    let output: String = TypstMarkup::new(typst_events.into_iter()).collect();
    
    println!("\n=== Final Output ===");
    println!("{}", output);
    
    // Count paragraphs
    let par_count = output.matches("#par()[").count();
    println!("\nParagraph count: {}", par_count);
}

