gsx Wasm 15f3837, Node.js v26.13.0, Intel(R) Core(TM) i9-10910 CPU @ 3.60GHz, 2026-11-06

gsx.wasm: 817996 bytes, 336611 in gzip -9.

Cold start: import 5.11 ms, first parse 6.55 ms.

| Case | UTF-16 units | Outcome | Parse median, ms | p95, ms | Peak RSS +MiB |
| --- | ---: | --- | ---: | ---: | ---: |
| typical ×91 | 112950 | accepted | 4.64 (0.05 per document) | 5.31 | 17.3 |
| long-4k | 3850 | accepted | 1.54 | 1.73 | 17.8 |
| long-32k | 32450 | accepted | 11.3 | 12.0 | 30.6 |
| long-128k | 130900 | accepted | 46.8 | 48.9 | 55.5 |
| prose-128k | 131057 | accepted | 3.72 | 3.95 | 17.5 |
| lines-text | 32766 | accepted | 23.2 | 23.7 | 29.3 |
| lines-blank | 24573 | accepted | 31.4 | 38.2 | 40.8 |
| lines-heading | 65532 | accepted | 60.5 | 68.2 | 60.8 |
| hard-breaks | 4093 | accepted | 2.87 | 3.14 | 17.5 |
| emphasis-delimiters | 129086 | accepted | 12.2 | 12.4 | 20.9 |
| label-starts | 120020 | accepted | 13.7 | 14.1 | 22.1 |
| escapes | 12285 | accepted | 8.07 | 8.63 | 30.4 |
| list-items | 32764 | accepted | 49.9 | 51.1 | 50.6 |
| container-depth | 128000 | accepted | 46.0 | 57.9 | 64.8 |
| table-cells | 27024 | accepted | 21.4 | 22.6 | 43.8 |
| paired-highlight | 6138 | accepted | 3.51 | 3.71 | 20.6 |
| paired-spoiler | 6138 | accepted | 3.52 | 3.71 | 20.9 |
| math-literals | 6138 | accepted | 1.79 | 1.87 | 13.9 |
| label-ends | 512 | accepted | 0.02 | 0.02 | 0.9 |
| escaped-code-ticks | 16384 | accepted | 9.29 | 9.87 | 29.8 |
| quote-paragraphs | 40955 | accepted | 44.4 | 45.2 | 44.2 |
| setext-headings | 32764 | accepted | 42.7 | 49.2 | 43.8 |
| nested-list-items | 40960 | accepted | 56.5 | 58.4 | 54.5 |
| html-comment-opens | 65538 | accepted | 9.08 | 9.23 | 19.8 |
| html-instruction-opens | 65535 | accepted | 8.26 | 8.44 | 16.7 |
| html-comment-tags | 65540 | accepted | 7.63 | 7.77 | 18.4 |
| code-tick-ramp | 125750 | accepted | 7.85 | 8.04 | 15.5 |
| http-run | 131072 | accepted | 5.91 | 6.13 | 18.6 |
| scheme-separator-run | 98304 | accepted | 8.18 | 8.35 | 18.2 |
| www-run | 131072 | accepted | 5.73 | 5.87 | 12.4 |
| reject-lines | 131072 | MARKDOWN_TOO_COMPLEX | 0.69 | 0.74 | 6.2 |
| reject-empty-items | 87380 | MARKDOWN_TOO_COMPLEX | 0.47 | 0.47 | 5.0 |
| reject-delimiters | 120000 | MARKDOWN_TOO_COMPLEX | 0.92 | 0.93 | 8.1 |
| reject-label-ends | 514 | MARKDOWN_TOO_COMPLEX | 0.02 | 0.02 | 0.9 |
