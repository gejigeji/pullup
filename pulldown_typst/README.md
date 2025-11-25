# pulldown_typst

This library is a pull parser for books created with
[Typst](https://github.com/typst/typst).

It does not currently do any parsing.

## Internal Link Support

This library now supports internal links in PDF documents. When converting Markdown to Typst:

1. **Heading Labels**: All headings automatically generate labels based on their text content. For example, a heading "My Section" will generate a label `<my-section>`.

2. **Internal Links**: Links that point to anchors (e.g., `#anchor` or `#My Section`) are automatically converted to Typst label references (e.g., `#link(<my-section>)`), enabling proper PDF navigation.

3. **External Links**: Links to external files or URLs remain unchanged and use the standard URL format.

### Example

Markdown:
```markdown
# My Heading

See [this section](#My Heading) for more details.
```

Generated Typst:
```typst
= My Heading <my-heading>

See #link(<my-heading>)[this section] for more details.
```

This ensures that when the Typst document is compiled to PDF, clicking on internal links will navigate to the correct section within the PDF.