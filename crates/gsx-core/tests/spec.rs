mod support;

use gsx_core::document::{Code, Details, Scheme, parse};

const EXAMPLES: &[(&str, &str, &str)] = &[
    (
        "commonmark 1",
        r"	foo	baz		bim
",
        r"<pre><code>foo	baz		bim
</code></pre>
",
    ),
    (
        "commonmark 2",
        r"  	foo	baz		bim
",
        r"<pre><code>foo	baz		bim
</code></pre>
",
    ),
    (
        "commonmark 4",
        r"  - foo

	bar
",
        r"<ul>
<li>
<p>foo</p>
<p>bar</p>
</li>
</ul>
",
    ),
    (
        "commonmark 5",
        r"- foo

		bar
",
        r"<ul>
<li>
<p>foo</p>
<pre><code>  bar
</code></pre>
</li>
</ul>
",
    ),
    (
        "commonmark 6",
        r">		foo
",
        r"<blockquote>
<pre><code>  foo
</code></pre>
</blockquote>
",
    ),
    (
        "commonmark 7",
        r"-		foo
",
        r"<ul>
<li>
<pre><code>  foo
</code></pre>
</li>
</ul>
",
    ),
    (
        "commonmark 8",
        r"    foo
	bar
",
        r"<pre><code>foo
bar
</code></pre>
",
    ),
    (
        "commonmark 9",
        r" - foo
   - bar
	 - baz
",
        r"<ul>
<li>foo
<ul>
<li>bar
<ul>
<li>baz</li>
</ul>
</li>
</ul>
</li>
</ul>
",
    ),
    (
        "commonmark 12",
        r#"\!\"\#\$\%\&\'\(\)\*\+\,\-\.\/\:\;\<\=\>\?\@\[\\\]\^\_\`\{\|\}\~
"#,
        r##"<p>!"#$%&amp;'()*+,-./:;&lt;=&gt;?@[\]^_`{|}~</p>
"##,
    ),
    (
        "commonmark 13",
        r"\	\A\a\ \3\φ\«
",
        r"<p>\	\A\a\ \3\φ\«</p>
",
    ),
    (
        "commonmark 14",
        r#"\*not emphasized*
\<br/> not a tag
\[not a link](/foo)
\`not code`
1\. not a list
\* not a list
\# not a heading
\[foo]: /url "not a reference"
\&ouml; not a character entity
"#,
        r#"<p>*not emphasized*
&lt;br/&gt; not a tag
[not a link](/foo)
`not code`
1. not a list
* not a list
# not a heading
[foo]: /url "not a reference"
&amp;ouml; not a character entity</p>
"#,
    ),
    (
        "commonmark 15",
        r"\\*emphasis*
",
        r"<p>\<em>emphasis</em></p>
",
    ),
    (
        "commonmark 16",
        r"foo\
bar
",
        r"<p>foo<br />
bar</p>
",
    ),
    (
        "commonmark 24",
        r"``` foo\+bar
foo
```
",
        r#"<pre><code class="language-foo+bar">foo
</code></pre>
"#,
    ),
    (
        "commonmark 25",
        r"&nbsp; &amp; &copy; &AElig; &Dcaron;
&frac34; &HilbertSpace; &DifferentialD;
&ClockwiseContourIntegral; &ngE;
",
        r"<p>  &amp; © Æ Ď
¾ ℋ ⅆ
∲ ≧̸</p>
",
    ),
    (
        "commonmark 26",
        r"&#35; &#1234; &#992; &#0;
",
        r"<p># Ӓ Ϡ �</p>
",
    ),
    (
        "commonmark 27",
        r"&#X22; &#XD06; &#xcab;
",
        r#"<p>" ആ ಫ</p>
"#,
    ),
    (
        "commonmark 28",
        r"&nbsp &x; &#; &#x;
&#87654321;
&#abcdef0;
&ThisIsNotDefined; &hi?;
",
        r"<p>&amp;nbsp &amp;x; &amp;#; &amp;#x;
&amp;#87654321;
&amp;#abcdef0;
&amp;ThisIsNotDefined; &amp;hi?;</p>
",
    ),
    (
        "commonmark 34",
        r"``` f&ouml;&ouml;
foo
```
",
        r#"<pre><code class="language-föö">foo
</code></pre>
"#,
    ),
    (
        "commonmark 42",
        r"- `one
- two`
",
        r"<ul>
<li>`one</li>
<li>two`</li>
</ul>
",
    ),
    (
        "commonmark 43",
        r"***
---
___
",
        r"<hr />
<hr />
<hr />
",
    ),
    (
        "commonmark 44",
        r"+++
",
        r"<p>+++</p>
",
    ),
    (
        "commonmark 45",
        r"===
",
        r"<p>===</p>
",
    ),
    (
        "commonmark 47",
        r" ***
  ***
   ***
",
        r"<hr />
<hr />
<hr />
",
    ),
    (
        "commonmark 57",
        r"- foo
***
- bar
",
        r"<ul>
<li>foo</li>
</ul>
<hr />
<ul>
<li>bar</li>
</ul>
",
    ),
    (
        "commonmark 59",
        r"Foo
---
bar
",
        r"<h2>Foo</h2>
<p>bar</p>
",
    ),
    (
        "commonmark 61",
        r"- Foo
- * * *
",
        r"<ul>
<li>Foo</li>
<li>
<hr />
</li>
</ul>
",
    ),
    (
        "commonmark 62",
        r"# foo
## foo
### foo
#### foo
##### foo
###### foo
",
        r"<h1>foo</h1>
<h2>foo</h2>
<h3>foo</h3>
<h4>foo</h4>
<h5>foo</h5>
<h6>foo</h6>
",
    ),
    (
        "commonmark 63",
        r"####### foo
",
        r"<p>####### foo</p>
",
    ),
    (
        "commonmark 64",
        r"#5 bolt

#hashtag
",
        r"<p>#5 bolt</p>
<p>#hashtag</p>
",
    ),
    (
        "commonmark 66",
        r"# foo *bar* \*baz\*
",
        r"<h1>foo <em>bar</em> *baz*</h1>
",
    ),
    (
        "commonmark 67",
        r"#                  foo                     
",
        r"<h1>foo</h1>
",
    ),
    (
        "commonmark 71",
        r"## foo ##
  ###   bar    ###
",
        r"<h2>foo</h2>
<h3>bar</h3>
",
    ),
    (
        "commonmark 73",
        r"### foo ###     
",
        r"<h3>foo</h3>
",
    ),
    (
        "commonmark 76",
        r"### foo \###
## foo #\##
# foo \#
",
        r"<h3>foo ###</h3>
<h2>foo ###</h2>
<h1>foo #</h1>
",
    ),
    (
        "commonmark 79",
        r"## 
#
### ###
",
        r"<h2></h2>
<h1></h1>
<h3></h3>
",
    ),
    (
        "commonmark 80",
        r"Foo *bar*
=========

Foo *bar*
---------
",
        r"<h1>Foo <em>bar</em></h1>
<h2>Foo <em>bar</em></h2>
",
    ),
    (
        "commonmark 81",
        r"Foo *bar
baz*
====
",
        r"<h1>Foo <em>bar
baz</em></h1>
",
    ),
    (
        "commonmark 86",
        r"Foo
   ----      
",
        r"<h2>Foo</h2>
",
    ),
    (
        "commonmark 93",
        r"> foo
bar
===
",
        r"<blockquote>
<p>foo
bar
===</p>
</blockquote>
",
    ),
    (
        "commonmark 97",
        r"
====
",
        r"<p>====</p>
",
    ),
    (
        "commonmark 98",
        r"---
---
",
        r"<hr />
<hr />
",
    ),
    (
        "commonmark 107",
        r"    a simple
      indented code block
",
        r"<pre><code>a simple
  indented code block
</code></pre>
",
    ),
    (
        "commonmark 108",
        r"  - foo

    bar
",
        r"<ul>
<li>
<p>foo</p>
<p>bar</p>
</li>
</ul>
",
    ),
    (
        "commonmark 110",
        r"    <a/>
    *hi*

    - one
",
        r"<pre><code>&lt;a/&gt;
*hi*

- one
</code></pre>
",
    ),
    (
        "commonmark 111",
        r"    chunk1

    chunk2
  
 
 
    chunk3
",
        r"<pre><code>chunk1

chunk2



chunk3
</code></pre>
",
    ),
    (
        "commonmark 112",
        r"    chunk1
      
      chunk2
",
        r"<pre><code>chunk1
  
  chunk2
</code></pre>
",
    ),
    (
        "commonmark 119",
        r"```
<
 >
```
",
        r"<pre><code>&lt;
 &gt;
</code></pre>
",
    ),
    (
        "commonmark 121",
        r"``
foo
``
",
        r"<p><code>foo</code></p>
",
    ),
    (
        "commonmark 123",
        r"~~~
aaa
```
~~~
",
        r"<pre><code>aaa
```
</code></pre>
",
    ),
    (
        "commonmark 127",
        r"`````

```
aaa
",
        r"<pre><code>
```
aaa
</code></pre>
",
    ),
    (
        "commonmark 129",
        r"```

  
```
",
        r"<pre><code>
  
</code></pre>
",
    ),
    (
        "commonmark 133",
        r"   ```
   aaa
    aaa
  aaa
   ```
",
        r"<pre><code>aaa
 aaa
aaa
</code></pre>
",
    ),
    (
        "commonmark 142",
        r"```ruby
def foo(x)
  return 3
end
```
",
        r#"<pre><code class="language-ruby">def foo(x)
  return 3
end
</code></pre>
"#,
    ),
    (
        "commonmark 143",
        r"~~~~    ruby startline=3 $%@#$
def foo(x)
  return 3
end
~~~~~~~
",
        r#"<pre><code class="language-ruby">def foo(x)
  return 3
end
</code></pre>
"#,
    ),
    (
        "commonmark 145",
        r"``` aa ```
foo
",
        r"<p><code>aa</code>
foo</p>
",
    ),
    (
        "commonmark 219",
        r"aaa

bbb
",
        r"<p>aaa</p>
<p>bbb</p>
",
    ),
    (
        "commonmark 220",
        r"aaa
bbb

ccc
ddd
",
        r"<p>aaa
bbb</p>
<p>ccc
ddd</p>
",
    ),
    (
        "commonmark 222",
        r"  aaa
 bbb
",
        r"<p>aaa
bbb</p>
",
    ),
    (
        "commonmark 226",
        r"aaa     
bbb     
",
        r"<p>aaa<br />
bbb</p>
",
    ),
    (
        "commonmark 227",
        r"  

aaa
  

# aaa

  
",
        r"<p>aaa</p>
<h1>aaa</h1>
",
    ),
    (
        "commonmark 228",
        r"> # Foo
> bar
> baz
",
        r"<blockquote>
<h1>Foo</h1>
<p>bar
baz</p>
</blockquote>
",
    ),
    (
        "commonmark 229",
        r"># Foo
>bar
> baz
",
        r"<blockquote>
<h1>Foo</h1>
<p>bar
baz</p>
</blockquote>
",
    ),
    (
        "commonmark 232",
        r"> # Foo
> bar
baz
",
        r"<blockquote>
<h1>Foo</h1>
<p>bar
baz</p>
</blockquote>
",
    ),
    (
        "commonmark 236",
        r">     foo
    bar
",
        r"<blockquote>
<pre><code>foo
</code></pre>
</blockquote>
<pre><code>bar
</code></pre>
",
    ),
    (
        "commonmark 238",
        r"> foo
    - bar
",
        r"<blockquote>
<p>foo
- bar</p>
</blockquote>
",
    ),
    (
        "commonmark 242",
        r"> foo

> bar
",
        r"<blockquote>
<p>foo</p>
</blockquote>
<blockquote>
<p>bar</p>
</blockquote>
",
    ),
    (
        "commonmark 252",
        r">     code

>    not code
",
        r"<blockquote>
<pre><code>code
</code></pre>
</blockquote>
<blockquote>
<p>not code</p>
</blockquote>
",
    ),
    (
        "commonmark 253",
        r"A paragraph
with two lines.

    indented code

> A block quote.
",
        r"<p>A paragraph
with two lines.</p>
<pre><code>indented code
</code></pre>
<blockquote>
<p>A block quote.</p>
</blockquote>
",
    ),
    (
        "commonmark 254",
        r"1.  A paragraph
    with two lines.

        indented code

    > A block quote.
",
        r"<ol>
<li>
<p>A paragraph
with two lines.</p>
<pre><code>indented code
</code></pre>
<blockquote>
<p>A block quote.</p>
</blockquote>
</li>
</ol>
",
    ),
    (
        "commonmark 256",
        r"- one

  two
",
        r"<ul>
<li>
<p>one</p>
<p>two</p>
</li>
</ul>
",
    ),
    (
        "commonmark 264",
        r"- Foo

      bar


      baz
",
        r"<ul>
<li>
<p>Foo</p>
<pre><code>bar


baz
</code></pre>
</li>
</ul>
",
    ),
    (
        "commonmark 273",
        r"1.     indented code

   paragraph

       more code
",
        r"<ol>
<li>
<pre><code>indented code
</code></pre>
<p>paragraph</p>
<pre><code>more code
</code></pre>
</li>
</ol>
",
    ),
    (
        "commonmark 278",
        r"-
  foo
-
  ```
  bar
  ```
-
      baz
",
        r"<ul>
<li>foo</li>
<li>
<pre><code>bar
</code></pre>
</li>
<li>
<pre><code>baz
</code></pre>
</li>
</ul>
",
    ),
    (
        "commonmark 280",
        r"-

  foo
",
        r"<ul>
<li></li>
</ul>
<p>foo</p>
",
    ),
    (
        "commonmark 282",
        r"- foo
-   
- bar
",
        r"<ul>
<li>foo</li>
<li></li>
<li>bar</li>
</ul>
",
    ),
    (
        "commonmark 293",
        r"> 1. > Blockquote
> continued here.
",
        r"<blockquote>
<ol>
<li>
<blockquote>
<p>Blockquote
continued here.</p>
</blockquote>
</li>
</ol>
</blockquote>
",
    ),
    (
        "commonmark 300",
        r"- # Foo
- Bar
  ---
  baz
",
        r"<ul>
<li>
<h1>Foo</h1>
</li>
<li>
<h2>Bar</h2>
baz</li>
</ul>
",
    ),
    (
        "commonmark 301",
        r"- foo
- bar
+ baz
",
        r"<ul>
<li>foo</li>
<li>bar</li>
</ul>
<ul>
<li>baz</li>
</ul>
",
    ),
    (
        "commonmark 303",
        r"Foo
- bar
- baz
",
        r"<p>Foo</p>
<ul>
<li>bar</li>
<li>baz</li>
</ul>
",
    ),
    (
        "commonmark 304",
        r"The number of windows in my house is
14.  The number of doors is 6.
",
        r"<p>The number of windows in my house is
14.  The number of doors is 6.</p>
",
    ),
    (
        "commonmark 306",
        r"- foo

- bar


- baz
",
        r"<ul>
<li>
<p>foo</p>
</li>
<li>
<p>bar</p>
</li>
<li>
<p>baz</p>
</li>
</ul>
",
    ),
    (
        "commonmark 307",
        r"- foo
  - bar
    - baz


      bim
",
        r"<ul>
<li>foo
<ul>
<li>bar
<ul>
<li>
<p>baz</p>
<p>bim</p>
</li>
</ul>
</li>
</ul>
</li>
</ul>
",
    ),
    (
        "commonmark 312",
        r"- a
 - b
  - c
   - d
    - e
",
        r"<ul>
<li>a</li>
<li>b</li>
<li>c</li>
<li>d
- e</li>
</ul>
",
    ),
    (
        "commonmark 315",
        r"* a
*

* c
",
        r"<ul>
<li>
<p>a</p>
</li>
<li></li>
<li>
<p>c</p>
</li>
</ul>
",
    ),
    (
        "commonmark 316",
        r"- a
- b

  c
- d
",
        r"<ul>
<li>
<p>a</p>
</li>
<li>
<p>b</p>
<p>c</p>
</li>
<li>
<p>d</p>
</li>
</ul>
",
    ),
    (
        "commonmark 318",
        r"- a
- ```
  b


  ```
- c
",
        r"<ul>
<li>a</li>
<li>
<pre><code>b


</code></pre>
</li>
<li>c</li>
</ul>
",
    ),
    (
        "commonmark 319",
        r"- a
  - b

    c
- d
",
        r"<ul>
<li>a
<ul>
<li>
<p>b</p>
<p>c</p>
</li>
</ul>
</li>
<li>d</li>
</ul>
",
    ),
    (
        "commonmark 325",
        r"* foo
  * bar

  baz
",
        r"<ul>
<li>
<p>foo</p>
<ul>
<li>bar</li>
</ul>
<p>baz</p>
</li>
</ul>
",
    ),
    (
        "commonmark 326",
        r"- a
  - b
  - c

- d
  - e
  - f
",
        r"<ul>
<li>
<p>a</p>
<ul>
<li>b</li>
<li>c</li>
</ul>
</li>
<li>
<p>d</p>
<ul>
<li>e</li>
<li>f</li>
</ul>
</li>
</ul>
",
    ),
    (
        "commonmark 328",
        r"`foo`
",
        r"<p><code>foo</code></p>
",
    ),
    (
        "commonmark 329",
        r"`` foo ` bar ``
",
        r"<p><code>foo ` bar</code></p>
",
    ),
    (
        "commonmark 330",
        r"` `` `
",
        r"<p><code>``</code></p>
",
    ),
    (
        "commonmark 335",
        r"``
foo
bar  
baz
``
",
        r"<p><code>foo bar   baz</code></p>
",
    ),
    (
        "commonmark 338",
        r"`foo\`bar`
",
        r"<p><code>foo\</code>bar`</p>
",
    ),
    (
        "commonmark 342",
        r"[not a `link](/foo`)
",
        r"<p>[not a <code>link](/foo</code>)</p>
",
    ),
    (
        "commonmark 350",
        r"*foo bar*
",
        r"<p><em>foo bar</em></p>
",
    ),
    (
        "commonmark 351",
        r"a * foo bar*
",
        r"<p>a * foo bar*</p>
",
    ),
    (
        "commonmark 352",
        r#"a*"foo"*
"#,
        r#"<p>a*"foo"*</p>
"#,
    ),
    (
        "commonmark 357",
        r"_foo bar_
",
        r"<p><em>foo bar</em></p>
",
    ),
    (
        "commonmark 360",
        r"foo_bar_
",
        r"<p>foo_bar_</p>
",
    ),
    (
        "commonmark 366",
        r"*foo bar *
",
        r"<p>*foo bar *</p>
",
    ),
    (
        "commonmark 376",
        r"_foo_bar_baz_
",
        r"<p><em>foo_bar_baz</em></p>
",
    ),
    (
        "commonmark 379",
        r"** foo bar**
",
        r"<p>** foo bar**</p>
",
    ),
    (
        "commonmark 392",
        r"**(**foo)
",
        r"<p>**(**foo)</p>
",
    ),
    (
        "commonmark 402",
        r"__foo__bar__baz__
",
        r"<p><strong>foo__bar__baz</strong></p>
",
    ),
    (
        "commonmark 411",
        r"*foo**bar**baz*
",
        r"<p><em>foo<strong>bar</strong>baz</em></p>
",
    ),
    (
        "commonmark 413",
        r"***foo** bar*
",
        r"<p><em><strong>foo</strong> bar</em></p>
",
    ),
    (
        "commonmark 418",
        r"*foo **bar *baz* bim** bop*
",
        r"<p><em>foo <strong>bar <em>baz</em> bim</strong> bop</em></p>
",
    ),
    (
        "commonmark 442",
        r"**foo*
",
        r"<p>*<em>foo</em></p>
",
    ),
    (
        "commonmark 445",
        r"****foo*
",
        r"<p>***<em>foo</em></p>
",
    ),
    (
        "commonmark 465",
        r"____foo____
",
        r"<p><strong><strong>foo</strong></strong></p>
",
    ),
    (
        "commonmark 468",
        r"_____foo_____
",
        r"<p><em><strong><strong>foo</strong></strong></em></p>
",
    ),
    (
        "commonmark 470",
        r"*foo __bar *baz bim__ bam*
",
        r"<p><em>foo <strong>bar *baz bim</strong> bam</em></p>
",
    ),
    (
        "commonmark 633",
        r"foo  
baz
",
        r"<p>foo<br />
baz</p>
",
    ),
    (
        "commonmark 634",
        r"foo\
baz
",
        r"<p>foo<br />
baz</p>
",
    ),
    (
        "commonmark 635",
        r"foo       
baz
",
        r"<p>foo<br />
baz</p>
",
    ),
    (
        "commonmark 646",
        r"### foo\
",
        r"<h3>foo\</h3>
",
    ),
    (
        "commonmark 648",
        r"foo
baz
",
        r"<p>foo
baz</p>
",
    ),
    (
        "commonmark 649",
        r"foo 
 baz
",
        r"<p>foo
baz</p>
",
    ),
    (
        "commonmark 650",
        r"hello $.;'there
",
        r"<p>hello $.;'there</p>
",
    ),
    (
        "commonmark 651",
        r"Foo χρῆν
",
        r"<p>Foo χρῆν</p>
",
    ),
    (
        "commonmark 652",
        r"Multiple     spaces
",
        r"<p>Multiple     spaces</p>
",
    ),
    (
        "gfm-table 1",
        r"| foo | bar |
| --- | --- |
| baz | bim |
",
        r"<table>
<thead>
<tr>
<th>foo</th>
<th>bar</th>
</tr>
</thead>
<tbody>
<tr>
<td>baz</td>
<td>bim</td>
</tr>
</tbody>
</table>
",
    ),
    (
        "gfm-table 2",
        r"| abc | defghi |
:-: | -----------:
bar | baz
",
        r#"<table>
<thead>
<tr>
<th style="text-align: center">abc</th>
<th style="text-align: right">defghi</th>
</tr>
</thead>
<tbody>
<tr>
<td style="text-align: center">bar</td>
<td style="text-align: right">baz</td>
</tr>
</tbody>
</table>
"#,
    ),
    (
        "gfm-table 3",
        r"| f\|oo  |
| ------ |
| b `\|` az |
| b **\|** im |
",
        r"<table>
<thead>
<tr>
<th>f|oo</th>
</tr>
</thead>
<tbody>
<tr>
<td>b <code>|</code> az</td>
</tr>
<tr>
<td>b <strong>|</strong> im</td>
</tr>
</tbody>
</table>
",
    ),
    (
        "gfm-table 4",
        r"| abc | def |
| --- | --- |
| bar | baz |
> bar
",
        r"<table>
<thead>
<tr>
<th>abc</th>
<th>def</th>
</tr>
</thead>
<tbody>
<tr>
<td>bar</td>
<td>baz</td>
</tr>
</tbody>
</table>
<blockquote>
<p>bar</p>
</blockquote>
",
    ),
    (
        "gfm-table 5",
        r"| abc | def |
| --- | --- |
| bar | baz |
bar

bar
",
        r"<table>
<thead>
<tr>
<th>abc</th>
<th>def</th>
</tr>
</thead>
<tbody>
<tr>
<td>bar</td>
<td>baz</td>
</tr>
<tr>
<td>bar</td>
<td></td>
</tr>
</tbody>
</table>
<p>bar</p>
",
    ),
    (
        "gfm-table 6",
        r"| abc | def |
| --- |
| bar |
",
        r"<p>| abc | def |
| --- |
| bar |</p>
",
    ),
    (
        "gfm-table 7",
        r"| abc | def |
| --- | --- |
| bar |
| bar | baz | boo |
",
        r"<table>
<thead>
<tr>
<th>abc</th>
<th>def</th>
</tr>
</thead>
<tbody>
<tr>
<td>bar</td>
<td></td>
</tr>
<tr>
<td>bar</td>
<td>baz</td>
</tr>
</tbody>
</table>
",
    ),
    (
        "gfm-table 8",
        r"| abc | def |
| --- | --- |
",
        r"<table>
<thead>
<tr>
<th>abc</th>
<th>def</th>
</tr>
</thead>
<tbody></tbody>
</table>
",
    ),
    (
        "gfm-strikethrough 2",
        r"This ~~has a

new paragraph~~.
",
        r"<p>This ~~has a</p>
<p>new paragraph~~.</p>
",
    ),
    (
        "gfm-tasklist 1",
        r"- [ ] foo
- [x] bar
",
        r#"<ul>
<li><input disabled="" type="checkbox"/>
foo</li>
<li><input disabled="" type="checkbox" checked=""/>
bar</li>
</ul>
"#,
    ),
    (
        "gfm-tasklist 2",
        r"- [x] foo
  - [ ] bar
  - [x] baz
- [ ] bim
",
        r#"<ul>
<li><input disabled="" type="checkbox" checked=""/>
foo
<ul>
<li><input disabled="" type="checkbox"/>
bar</li>
<li><input disabled="" type="checkbox" checked=""/>
baz</li>
</ul>
</li>
<li><input disabled="" type="checkbox"/>
bim</li>
</ul>
"#,
    ),
];

const DEVIATIONS: &[(&str, &str, &str)] = &[
    (
        "gfm-strikethrough 1: single tilde is not strikethrough",
        "~~Hi~~ Hello, ~there~ world!\n",
        "<p><del>Hi</del> Hello, ~there~ world!</p>\n",
    ),
    ("commonmark 168: raw HTML is literal text", "<del>*foo*</del>\n", "<p>&lt;del&gt;*foo*&lt;/del&gt;</p>\n"),
    (
        "commonmark 192: definitions and references are literal text",
        "[foo]: /url \"title\"\n\n[foo]\n",
        "[foo]: /url \"title\"\n<p>[foo]</p>\n",
    ),
    ("commonmark 483: relative links are literal text", "[link](/uri)\n", "<p>[link](/uri)</p>\n"),
    ("commonmark 572: images are literal text", "![foo](/url \"title\")\n", "<p>![foo](/url \"title\")</p>\n"),
    (
        "commonmark 594: angle autolinks link to the serialized WHATWG href",
        "<http://foo.bar.baz>\n",
        "<p><a href=\"http://foo.bar.baz/\">http://foo.bar.baz</a></p>\n",
    ),
    (
        "gfm autolink literal: bare URLs stay text",
        "Visit www.commonmark.org/help.\n",
        "<p>Visit www.commonmark.org/help.</p>\n",
    ),
];

fn check(cases: &[(&str, &str, &str)]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(name, markdown, expected)| {
            let document = parse(markdown, &[]).unwrap_or_else(|error| panic!("{name}: {error}"));
            let actual = support::html::render(&document);
            (actual != *expected)
                .then(|| format!("{name} {markdown:?}\n    expected {expected:?}\n    actual   {actual:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), cases.len(), failures.join("\n"));
}

#[test]
fn matches_commonmark_and_gfm_examples() {
    check(EXAMPLES);
}

#[test]
fn records_gsx_deviations() {
    check(DEVIATIONS);
}

#[test]
fn reports_literalized_links() {
    let document = parse("[link](/uri) [mail](mailto:a@b.c) [ok](https://example.com)\n", &[]).unwrap();
    let schemes: Vec<_> =
        document.diagnostics().iter().map(|diagnostic| (diagnostic.code, diagnostic.details.clone())).collect();
    assert_eq!(
        schemes,
        [
            (Code::LinkLiteralized, Details::Scheme(Scheme::Relative)),
            (Code::LinkLiteralized, Details::Scheme(Scheme::Unsupported)),
        ]
    );
}
