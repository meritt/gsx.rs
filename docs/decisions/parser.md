# Parser ownership

`gsx-core` owns the tokenizer and grammar as first-party code, without a parser crate dependency or periodic upstream synchronization. Positioned events provide content, marker ranges and literal provenance; the document model is built from those events rather than a generic Markdown AST.

| Decision | Constraint |
| --- | --- |
| Maintain the tokenizer in-tree | Paired delimiters, time directives, media and table spoilers affect state transitions, dispatch and attention; a parser dependency would still require grammar changes |
| Enforce budgets during tokenization and event validation | Excessive structures must be rejected before later stages traverse them |
| Recognize extensions inside the grammar | A separate regex pass cannot preserve delimiter precedence, container context and source ranges |
| Keep syntax and document conversion separate | Unsupported constructs can retain source spelling while the public model contains only supported node kinds |

The tokenizer includes the CommonMark/GFM constructs and generated punctuation and character-reference tables it uses. HTML/MDX output compilers and an external AST are outside the core. The retained [MIT notice](../../crates/gsx-core/LICENSE-markdown-rs) covers the markdown-rs code.

[Conformance](conformance.md) defines the accepted syntax and rejection contract.
