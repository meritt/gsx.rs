gsx Wasm fdb4eb6, Node.js v26.13.0, Intel(R) Core(TM) i9-10910 CPU @ 3.60GHz, 2026-11-12

gsx.wasm: 870924 bytes, 360957 in gzip -9.

Cold start: import 5.68 ms, first parse 6.75 ms.

| Case | UTF-16 units | Outcome | Parse median, ms | p95, ms | Peak RSS +MiB |
| --- | ---: | --- | ---: | ---: | ---: |
| typical ×91 | 112952 | accepted | 5.00 (0.06 per document) | 5.88 | 21.2 |
| long-4k | 3857 | accepted | 1.58 | 1.73 | 15.4 |
| long-32k | 32509 | accepted | 11.8 | 12.6 | 34.2 |
| long-128k | 130587 | accepted | 47.8 | 55.9 | 53.0 |
| prose-128k | 131057 | accepted | 3.97 | 4.25 | 16.7 |
| prose-256k | 262114 | accepted | 9.00 | 9.88 | 22.3 |
| units | 262144 | accepted | 3.31 | 3.46 | 15.3 |
| inline-depth | 67000 | accepted | 38.0 | 45.7 | 48.0 |
| lines-text | 32766 | accepted | 24.1 | 24.5 | 27.4 |
| lines-blank | 24573 | accepted | 31.7 | 33.1 | 35.4 |
| lines-heading | 65532 | accepted | 61.4 | 62.7 | 56.5 |
| hard-breaks | 4093 | accepted | 2.94 | 3.17 | 16.8 |
| emphasis-delimiters | 129086 | accepted | 12.6 | 12.9 | 19.9 |
| label-starts | 120020 | accepted | 14.4 | 14.7 | 22.7 |
| escapes | 12285 | accepted | 8.57 | 8.93 | 28.3 |
| list-items | 32764 | accepted | 51.5 | 60.8 | 55.1 |
| container-depth | 128000 | accepted | 48.1 | 52.0 | 60.4 |
| table-cells | 27024 | accepted | 21.9 | 22.5 | 41.3 |
| paired-highlight | 6138 | accepted | 3.77 | 3.88 | 20.1 |
| paired-spoiler | 6138 | accepted | 3.68 | 3.83 | 20.6 |
| math-literals | 6138 | accepted | 1.49 | 1.54 | 14.9 |
| label-ends | 512 | accepted | 0.02 | 0.02 | 1.1 |
| escaped-code-ticks | 16384 | accepted | 9.61 | 10.1 | 28.9 |
| quote-paragraphs | 40955 | accepted | 46.1 | 53.6 | 57.6 |
| setext-headings | 32764 | accepted | 44.5 | 45.8 | 40.6 |
| nested-list-items | 40960 | accepted | 59.3 | 68.5 | 53.5 |
| html-block-opens | 40955 | accepted | 18.5 | 19.4 | 30.3 |
| media-directive-line | 262140 | accepted | 37.4 | 49.2 | 63.7 |
| time-label-opens | 262144 | accepted | 169 | 169 | 37.8 |
| definition-references | 260933 | accepted | 141 | 149 | 83.8 |
| footnote-references | 261305 | accepted | 130 | 134 | 81.1 |
| label-normalization | 262110 | accepted | 3.69 | 3.81 | 15.9 |
| html-comment-opens | 65538 | accepted | 9.67 | 9.84 | 19.4 |
| html-instruction-opens | 65535 | accepted | 8.78 | 9.15 | 16.5 |
| html-comment-tags | 65540 | accepted | 8.13 | 8.32 | 17.6 |
| code-tick-ramp | 125750 | accepted | 8.81 | 9.00 | 14.4 |
| http-run | 131072 | accepted | 6.21 | 6.34 | 18.1 |
| scheme-separator-run | 98304 | accepted | 7.41 | 7.59 | 18.1 |
| www-run | 131072 | accepted | 6.26 | 6.36 | 12.1 |
| autolink-trail-path | 262012 | accepted | 18.9 | 20.7 | 12.1 |
| autolink-trail-domain | 262006 | accepted | 19.0 | 21.0 | 11.8 |
| rich-captions | 261401 | accepted | 31.3 | 36.3 | 41.5 |
| bluesky-parens | 262015 | accepted | 3.47 | 3.61 | 15.7 |
| entity-cap | 261452 | accepted | 305 | 331 | 168.7 |
| bluesky-tags | 262000 | accepted | 22.0 | 22.6 | 41.4 |
| bluesky-mentions | 261993 | accepted | 19.6 | 20.8 | 36.4 |
| reject-lines | 131072 | MARKDOWN_TOO_COMPLEX | 0.68 | 0.73 | 6.1 |
| reject-empty-items | 87380 | MARKDOWN_TOO_COMPLEX | 0.46 | 0.48 | 5.2 |
| reject-delimiters | 120000 | MARKDOWN_TOO_COMPLEX | 0.93 | 0.98 | 7.2 |
| reject-label-ends | 514 | MARKDOWN_TOO_COMPLEX | 0.02 | 0.02 | 0.9 |
| reject-units | 262146 | MARKDOWN_TOO_COMPLEX | 0.01 | 0.01 | 0.5 |
| reject-inline-depth | 69 | MARKDOWN_TOO_COMPLEX | 0.03 | 0.03 | 0.9 |
