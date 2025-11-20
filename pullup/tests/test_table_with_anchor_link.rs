#[cfg(all(feature = "markdown", feature = "typst"))]
#[test]
fn test_table_with_anchor_link_and_paragraph() {
    use pulldown_cmark::Parser;
    use pullup::markdown::MarkdownIter;
    use pullup::markdown::to::typst::*;
    use pulldown_typst::markup::TypstMarkup;
    
    // User's actual scenario with table and anchor link
    let md = "## 消息头

| 位置 | 字段 | 类型 | 长度 | 说明 |
| :---- | :---- | :---- | :---- | :---- |
| 1 | MAGIC | UINT16 | 2 | 报文开始标识符 固定值：0x53,0x50 |
| 2 | 消息序列号 | UINT32 | 4 | 消息序列号，用于消息跟踪、调试，需要保证单个连接在4小时内序列号不重复 |
| 3 | 协议版本号 | UByte | 1 | 当前通讯协议版本号 |
| 4 | 机器类型 | UByte | 1 | 查看[分拣机机器类型表](#附录五-分拣机机器类型表) |
| 5 | 消息指令 | UINT16 | 2 | 消息指令，查看6.3章节 |
| 6 | 消息体长度 | UINT16 | 2 | 消息体payload长度 |
| 7 | 是否需要ACK | UByte | 1 | 该条消息是否需要ack |
| 8 | Reserved字段 | Byte[] | 8 | 保留字段，统一填充0x00 |
| 9 | 校验位 | Byte | 1 | 除了校验位外所有字节(消息头+消息体)的异或结果 |

Ack消息的重发逻辑：  

标记为需要ack的消息发出后，接收方需要在100ms内进行消息的ack回复，如果发送方在100ms内未接收到ack消息，发送方需要自动重发当前消息，消息的消息序列号保持不变；持续重试3次后，如果无法送达，作丢弃处理";
    
    // IMPORTANT: MergeConsecutiveParagraphs MUST be used!
    // The correct converter chain should include MergeConsecutiveParagraphs
    // Also need ConvertHeadings to convert headings properly
    let events = MergeConsecutiveParagraphs::new(
        ConvertLinks::new(
            ConvertImages::new(
                ConvertText::new(
                    ConvertHardBreaks::new(
                        ConvertSoftBreaks::new(
                            ConvertParagraphs::new(
                                ConvertHeadings::new(
                                    ConvertTables::new(
                                        MarkdownIter(Parser::new_ext(
                                            &md,
                                            pulldown_cmark::Options::ENABLE_TABLES,
                                        ))
                                    )
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
    
    // Check that anchor link uses <label> syntax, not string
    assert!(output.contains("#link(<附录五-分拣机机器类型表>)"), 
        "Anchor link should use <label> syntax, not string. Output:\n{}", output);
    assert!(!output.contains("#link(\"#附录五-分拣机机器类型表\")"), 
        "Anchor link should NOT use string syntax. Output:\n{}", output);
    
    // Check that paragraphs are properly merged (should have only ONE paragraph after table)
    let par_count = output.matches("#par()[").count();
    println!("Paragraph count: {}", par_count);
    
    // Should NOT have nested #par()[]
    assert!(!output.contains("#par()[标记为需要ack的消息发出后"), 
        "Should not have separate paragraph for content after linebreak. Output:\n{}", output);
    
    // Should have the paragraph with linebreak
    assert!(output.contains("#par()[Ack消息的重发逻辑："), 
        "Should have paragraph with title. Output:\n{}", output);
}

