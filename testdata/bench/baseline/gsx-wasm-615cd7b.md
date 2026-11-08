gsx Wasm 615cd7b (dirty), Node.js v26.13.0, Apple M1 Pro, 2026-11-08

gsx.wasm: 858641 bytes, 356047 in gzip -9.

Cold start: import 3.52 ms, first parse 5.36 ms.

| Case | UTF-16 units | Outcome | Parse median, ms | p95, ms | Peak RSS +MiB |
| --- | ---: | --- | ---: | ---: | ---: |
| typical ×91 | 112950 | accepted | 3.56 (0.04 per document) | 4.08 | 20.3 |
| long-4k | 3850 | accepted | 1.24 | 1.46 | 17.3 |
| long-32k | 32450 | accepted | 9.34 | 10.1 | 21.5 |
| long-128k | 130900 | accepted | 37.4 | 41.7 | 42.4 |
| prose-128k | 131057 | accepted | 3.79 | 4.03 | 16.0 |
| units | 262144 | accepted | 3.46 | 4.03 | 17.5 |
| inline-depth | 67000 | accepted | 30.5 | 34.0 | 44.4 |
| lines-text | 32766 | accepted | 19.8 | 20.3 | 25.8 |
| lines-blank | 24573 | accepted | 24.9 | 25.1 | 28.5 |
| lines-heading | 65532 | accepted | 45.3 | 48.5 | 46.8 |
| hard-breaks | 4093 | accepted | 2.28 | 2.42 | 16.2 |
| emphasis-delimiters | 129086 | accepted | 10.9 | 12.2 | 17.2 |
| label-starts | 120020 | accepted | 11.3 | 11.5 | 19.3 |
| escapes | 12285 | accepted | 6.30 | 6.80 | 28.2 |
| list-items | 32764 | accepted | 40.8 | 44.3 | 48.0 |
| container-depth | 128000 | accepted | 33.5 | 38.6 | 56.3 |
| table-cells | 27024 | accepted | 17.1 | 18.5 | 33.3 |
| paired-highlight | 6138 | accepted | 2.74 | 2.87 | 23.7 |
| paired-spoiler | 6138 | accepted | 2.74 | 2.86 | 23.3 |
| math-literals | 6138 | accepted | 1.40 | 1.50 | 15.1 |
| label-ends | 512 | accepted | 0.01 | 0.01 | 1.0 |
| escaped-code-ticks | 16384 | accepted | 7.20 | 7.45 | 28.6 |
| quote-paragraphs | 40955 | accepted | 37.6 | 39.2 | 36.5 |
| setext-headings | 32764 | accepted | 34.8 | 37.6 | 37.8 |
| nested-list-items | 40960 | accepted | 47.2 | 49.5 | 49.1 |
| html-comment-opens | 65538 | accepted | 8.13 | 8.49 | 18.8 |
| html-instruction-opens | 65535 | accepted | 7.31 | 7.54 | 15.9 |
| html-comment-tags | 65540 | accepted | 6.55 | 6.74 | 15.6 |
| code-tick-ramp | 125750 | accepted | 7.13 | 7.34 | 13.3 |
| http-run | 131072 | accepted | 5.13 | 5.59 | 19.6 |
| scheme-separator-run | 98304 | accepted | 6.87 | 7.11 | 20.6 |
| www-run | 131072 | accepted | 4.74 | 4.91 | 11.7 |
| reject-lines | 131072 | MARKDOWN_TOO_COMPLEX | 0.51 | 0.55 | 6.7 |
| reject-empty-items | 87380 | MARKDOWN_TOO_COMPLEX | 0.35 | 0.37 | 5.7 |
| reject-delimiters | 120000 | MARKDOWN_TOO_COMPLEX | 0.81 | 0.91 | 8.7 |
| reject-label-ends | 514 | MARKDOWN_TOO_COMPLEX | 0.01 | 0.01 | 1.0 |
| reject-units | 262146 | MARKDOWN_TOO_COMPLEX | 0.48 | 0.52 | 4.2 |
| reject-inline-depth | 69 | MARKDOWN_TOO_COMPLEX | 0.01 | 0.02 | 0.9 |
