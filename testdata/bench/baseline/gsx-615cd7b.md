gsx 615cd7b (dirty), native, rustc 1.99.1 (58cf8f9c3 2026-10-13), , 2026-11-08

| Case | UTF-16 units | Outcome | Parse median, ms | p95, ms | Peak heap +MiB |
| --- | ---: | --- | ---: | ---: | ---: |
| typical ×91 | 112950 | accepted | 1.52 (0.02 per document) | 1.63 | 0.2 |
| long-4k | 3850 | accepted | 0.43 | 0.44 | 0.5 |
| long-32k | 32450 | accepted | 3.54 | 3.81 | 4.0 |
| long-128k | 130900 | accepted | 14.9 | 15.4 | 16.1 |
| prose-128k | 131057 | accepted | 1.99 | 2.02 | 0.9 |
| units | 262144 | accepted | 1.68 | 1.70 | 0.8 |
| inline-depth | 67000 | accepted | 10.7 | 12.1 | 21.3 |
| lines-text | 32766 | accepted | 8.03 | 8.11 | 14.1 |
| lines-blank | 24573 | accepted | 9.76 | 9.96 | 12.0 |
| lines-heading | 65532 | accepted | 18.0 | 18.4 | 31.4 |
| hard-breaks | 4093 | accepted | 0.72 | 0.73 | 1.2 |
| emphasis-delimiters | 129086 | accepted | 4.83 | 4.88 | 0.5 |
| label-starts | 120020 | accepted | 5.38 | 5.53 | 3.3 |
| escapes | 12285 | accepted | 1.44 | 1.46 | 4.0 |
| list-items | 32764 | accepted | 15.2 | 15.5 | 20.9 |
| container-depth | 128000 | accepted | 14.7 | 15.4 | 42.6 |
| table-cells | 27024 | accepted | 6.65 | 6.92 | 11.4 |
| paired-highlight | 6138 | accepted | 0.86 | 0.87 | 1.9 |
| paired-spoiler | 6138 | accepted | 0.86 | 0.89 | 1.9 |
| math-literals | 6138 | accepted | 0.42 | 0.42 | 0.8 |
| label-ends | 512 | accepted | 0.01 | 0.01 | 0.0 |
| escaped-code-ticks | 16384 | accepted | 1.82 | 1.85 | 4.0 |
| quote-paragraphs | 40955 | accepted | 14.5 | 15.0 | 21.7 |
| setext-headings | 32764 | accepted | 14.2 | 14.6 | 17.8 |
| nested-list-items | 40960 | accepted | 17.7 | 18.4 | 24.2 |
| html-comment-opens | 65538 | accepted | 2.98 | 3.03 | 9.1 |
| html-instruction-opens | 65535 | accepted | 2.43 | 2.48 | 4.6 |
| html-comment-tags | 65540 | accepted | 2.27 | 2.29 | 4.6 |
| code-tick-ramp | 125750 | accepted | 2.56 | 2.56 | 0.4 |
| http-run | 131072 | accepted | 2.13 | 2.24 | 4.6 |
| scheme-separator-run | 98304 | accepted | 3.09 | 3.13 | 9.1 |
| www-run | 131072 | accepted | 2.21 | 2.24 | 0.3 |
| reject-lines | 131072 | MARKDOWN_TOO_COMPLEX | 0.32 | 0.33 | 0.8 |
| reject-empty-items | 87380 | MARKDOWN_TOO_COMPLEX | 0.22 | 0.22 | 0.4 |
| reject-delimiters | 120000 | MARKDOWN_TOO_COMPLEX | 0.59 | 0.60 | 0.7 |
| reject-label-ends | 514 | MARKDOWN_TOO_COMPLEX | 0.00 | 0.00 | 0.0 |
| reject-units | 262146 | MARKDOWN_TOO_COMPLEX | 0.24 | 0.24 | 0.0 |
| reject-inline-depth | 69 | MARKDOWN_TOO_COMPLEX | 0.01 | 0.01 | 0.0 |

Projection median, ms; * fitted to the limit.

| Target | long-4k | long-32k | long-128k | prose-128k |
| --- | ---: | ---: | ---: | ---: |
| x-plain | 0.25* | 1.18* | 1.99* | 1.48* |
| mastodon-plain | 0.57* | 2.27* | 8.39* | 1.32* |
| linkedin-plain | 0.04 | 1.04* | 2.45* | 1.52* |
| facebook-plain | 0.02 | 0.13 | 4.93* | 2.28* |
| instagram-plain | 0.46* | 0.87* | 2.04* | 0.85* |
| vk-plain | 0.02 | 0.13 | 0.51 | 0.03 |
| youtube-plain | 0.03 | 0.89* | 2.05* | 1.33* |
| tiktok-plain | 0.02 | 0.16 | 0.64 | 0.19 |
| slack-plain | 0.02 | 0.19 | 0.76 | 0.40 |
| bluesky-entities | 0.15* | 0.70* | 2.64* | 2.19* |
| threads-entities | 0.49* | 1.52* | 2.65* | 1.69* |
| telegram-entities | 0.06* | 0.26* | 1.02* | 0.05* |
| telegram-html | 0.05 | 1.58* | 3.10* | 0.80* |
| telegram-html-rich | 0.06 | 0.45 | 10.3* | 2.37* |
| max-html | 0.04 | 0.95* | 2.20* | 0.77* |
| discord-markdown | 0.34* | 0.96* | 3.47* | 1.67* |
| slack-markdown | 0.06 | 3.01* | 5.70* | 2.42* |
| discord-blocks | 0.05 | 1.30* | 3.85* | 1.83* |
| slack-blocks | 0.06 | 3.01* | 6.07* | 2.39* |
| tiktok-plain-photo | 0.02 | 0.83* | 1.73* | 1.27* |
