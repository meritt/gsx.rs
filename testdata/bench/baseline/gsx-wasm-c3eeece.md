gsx Wasm c3eeece, Node.js v26.13.0, Intel(R) Core(TM) i9-10910 CPU @ 3.60GHz, 2026-11-12

gsx.wasm: 876544 bytes, 364247 in gzip -9.

Cold start: import 5.55 ms, first parse 6.46 ms.

| Case | UTF-16 units | Outcome | Parse median, ms | p95, ms | Peak RSS +MiB |
| --- | ---: | --- | ---: | ---: | ---: |
| typical ×91 | 112952 | accepted | 4.87 (0.05 per document) | 5.70 | 20.2 |
| long-4k | 3857 | accepted | 1.54 | 1.70 | 16.4 |
| long-32k | 32509 | accepted | 11.5 | 12.8 | 32.0 |
| long-128k | 130587 | accepted | 47.6 | 53.5 | 54.5 |
| prose-128k | 131057 | accepted | 3.97 | 4.41 | 16.2 |
| prose-256k | 262114 | accepted | 9.04 | 9.46 | 20.8 |
| units | 262144 | accepted | 3.29 | 3.48 | 16.0 |
| inline-depth | 67000 | accepted | 35.8 | 44.8 | 49.6 |
| lines-text | 32766 | accepted | 23.0 | 28.0 | 28.3 |
| lines-blank | 24573 | accepted | 31.9 | 33.7 | 36.1 |
| lines-heading | 65532 | accepted | 58.1 | 61.1 | 60.6 |
| hard-breaks | 4093 | accepted | 2.77 | 3.50 | 17.2 |
| emphasis-delimiters | 129086 | accepted | 12.5 | 13.5 | 19.0 |
| label-starts | 120020 | accepted | 14.2 | 17.0 | 23.1 |
| escapes | 12285 | accepted | 9.09 | 10.8 | 28.3 |
| list-items | 32764 | accepted | 53.8 | 56.7 | 56.8 |
| container-depth | 128000 | accepted | 49.7 | 67.0 | 62.3 |
| table-cells | 27024 | accepted | 21.9 | 23.4 | 43.0 |
| paired-highlight | 6138 | accepted | 3.54 | 3.90 | 19.6 |
| paired-spoiler | 6138 | accepted | 3.49 | 3.67 | 18.8 |
| math-literals | 6138 | accepted | 1.43 | 1.57 | 13.7 |
| label-ends | 512 | accepted | 0.02 | 0.02 | 0.9 |
| escaped-code-ticks | 16384 | accepted | 9.24 | 9.60 | 29.0 |
| quote-paragraphs | 40955 | accepted | 48.0 | 59.5 | 53.3 |
| setext-headings | 32764 | accepted | 45.5 | 47.3 | 46.0 |
| nested-list-items | 40960 | accepted | 66.1 | 69.6 | 54.8 |
| html-block-opens | 40955 | accepted | 21.4 | 24.2 | 30.6 |
| media-directive-line | 262140 | accepted | 36.4 | 47.4 | 62.1 |
| time-label-opens | 262144 | accepted | 172 | 188 | 35.0 |
| definition-references | 260933 | accepted | 141 | 143 | 84.5 |
| footnote-references | 261305 | accepted | 131 | 131 | 88.2 |
| label-normalization | 262110 | accepted | 3.73 | 3.91 | 16.8 |
| html-comment-opens | 65538 | accepted | 9.21 | 9.72 | 19.2 |
| html-instruction-opens | 65535 | accepted | 8.38 | 8.61 | 16.2 |
| html-comment-tags | 65540 | accepted | 7.84 | 8.15 | 18.5 |
| code-tick-ramp | 125750 | accepted | 8.15 | 9.28 | 14.5 |
| http-run | 131072 | accepted | 6.04 | 6.30 | 17.9 |
| scheme-separator-run | 98304 | accepted | 7.22 | 7.48 | 16.7 |
| www-run | 131072 | accepted | 5.94 | 6.26 | 10.8 |
| autolink-trail-path | 262012 | accepted | 18.9 | 19.4 | 12.4 |
| autolink-trail-domain | 262006 | accepted | 18.5 | 19.0 | 11.7 |
| rich-captions | 261401 | accepted | 30.0 | 40.4 | 43.6 |
| bluesky-parens | 262015 | accepted | 3.53 | 3.74 | 16.0 |
| entity-cap | 261452 | accepted | 283 | 321 | 170.8 |
| bluesky-tags | 262000 | accepted | 21.9 | 25.0 | 39.1 |
| bluesky-mentions | 261993 | accepted | 19.5 | 21.0 | 38.8 |
| reject-lines | 131072 | MARKDOWN_TOO_COMPLEX | 0.66 | 0.77 | 5.7 |
| reject-empty-items | 87380 | MARKDOWN_TOO_COMPLEX | 0.45 | 0.51 | 5.1 |
| reject-delimiters | 120000 | MARKDOWN_TOO_COMPLEX | 0.95 | 1.05 | 8.2 |
| reject-label-ends | 514 | MARKDOWN_TOO_COMPLEX | 0.02 | 0.02 | 0.9 |
| reject-units | 262146 | MARKDOWN_TOO_COMPLEX | 0.01 | 0.01 | 0.4 |
| reject-inline-depth | 69 | MARKDOWN_TOO_COMPLEX | 0.03 | 0.03 | 1.1 |
