use mdbook::errors::Result as MdbookResult;
use pulldown_cmark::{CodeBlockKind::*, Event, Options, Parser, Tag};

pub use crate::preprocessor::Admonish;
use crate::{
    book_config::OnFailure,
    parse::parse_admonition,
    types::{AdmonitionDefaults, RenderTextMode},
};

pub(crate) fn preprocess(
    content: &str,
    on_failure: OnFailure,
    admonition_defaults: &AdmonitionDefaults,
    render_text_mode: RenderTextMode,
) -> MdbookResult<String> {
    let mut id_counter = Default::default();
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut admonish_blocks = vec![];

    let events = Parser::new_ext(content, opts);

    for (event, span) in events.into_offset_iter() {
        if let Event::Start(Tag::CodeBlock(Fenced(info_string))) = event.clone() {
            let span_content = &content[span.start..span.end];

            let admonition = match parse_admonition(
                info_string.as_ref(),
                admonition_defaults,
                span_content,
                on_failure,
            ) {
                Some(admonition) => admonition,
                None => continue,
            };

            let admonition = admonition?;

            // Once we've identitified admonition blocks, handle them differently
            // depending on our render mode
            let new_content = match render_text_mode {
                RenderTextMode::Html => admonition.html_with_unique_ids(&mut id_counter),
                RenderTextMode::Strip => admonition.strip(),
            };

            admonish_blocks.push((span, new_content));
        }
    }

    let mut content = content.to_string();
    for (span, block) in admonish_blocks.iter().rev() {
        let pre_content = &content[..span.start];
        let post_content = &content[span.end..];
        content = format!("{}{}{}", pre_content, block, post_content);
    }

    Ok(content)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn prep(content: &str) -> String {
        preprocess(
            content,
            OnFailure::Continue,
            &AdmonitionDefaults::default(),
            RenderTextMode::Html,
        )
        .unwrap()
    }

    #[test]
    fn adds_admonish() {
        let content = r#"# Chapter
```admonish
A simple admonition.
```
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-note" class="admonition note">
<div class="admonition-title">

Note

<a class="admonition-anchor-link" href="#admonition-note"></a>
</div>
<div>

A simple admonition.

</div>
</div>
Text
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn adds_admonish_longer_code_fence() {
        let content = r#"# Chapter
````admonish
```json
{}
```
````
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-note" class="admonition note">
<div class="admonition-title">

Note

<a class="admonition-anchor-link" href="#admonition-note"></a>
</div>
<div>

```json
{}
```

</div>
</div>
Text
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn adds_admonish_directive() {
        let content = r#"# Chapter
```admonish warning
A simple admonition.
```
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-warning" class="admonition warning">
<div class="admonition-title">

Warning

<a class="admonition-anchor-link" href="#admonition-warning"></a>
</div>
<div>

A simple admonition.

</div>
</div>
Text
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn adds_admonish_directive_alternate() {
        let content = r#"# Chapter
```admonish caution
A warning with alternate title.
```
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-caution" class="admonition warning">
<div class="admonition-title">

Caution

<a class="admonition-anchor-link" href="#admonition-caution"></a>
</div>
<div>

A warning with alternate title.

</div>
</div>
Text
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn adds_admonish_directive_title() {
        let content = r#"# Chapter
```admonish warning "Read **this**!"
A simple admonition.
```
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-read-this" class="admonition warning">
<div class="admonition-title">

Read **this**!

<a class="admonition-anchor-link" href="#admonition-read-this"></a>
</div>
<div>

A simple admonition.

</div>
</div>
Text
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn leaves_tables_untouched() {
        // Regression test.
        // Previously we forgot to enable the same markdwon extensions as mdbook itself.

        let content = r#"# Heading
| Head 1 | Head 2 |
|--------|--------|
| Row 1  | Row 2  |
"#;

        let expected = r#"# Heading
| Head 1 | Head 2 |
|--------|--------|
| Row 1  | Row 2  |
"#;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn leaves_html_untouched() {
        // Regression test.
        // Don't remove important newlines for syntax nested inside HTML

        let content = r#"# Heading
<del>
*foo*
</del>
"#;

        let expected = r#"# Heading
<del>
*foo*
</del>
"#;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn html_in_list() {
        // Regression test.
        // Don't remove important newlines for syntax nested inside HTML

        let content = r#"# Heading
1. paragraph 1
   ```
   code 1
   ```
2. paragraph 2
"#;

        let expected = r#"# Heading
1. paragraph 1
   ```
   code 1
   ```
2. paragraph 2
"#;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn info_string_that_changes_length_when_parsed() {
        let content = r#"
```admonish note "And \\"<i>in</i>\\" the title"
With <b>html</b> styling.
```
hello
"#;

        let expected = r##"

<div id="admonition-and-in-the-title" class="admonition note">
<div class="admonition-title">

And "<i>in</i>" the title

<a class="admonition-anchor-link" href="#admonition-and-in-the-title"></a>
</div>
<div>

With <b>html</b> styling.

</div>
</div>
hello
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn info_string_ending_in_symbol() {
        let content = r#"
```admonish warning "Trademark™"
Should be respected
```
hello
"#;

        let expected = r##"

<div id="admonition-trademark" class="admonition warning">
<div class="admonition-title">

Trademark™

<a class="admonition-anchor-link" href="#admonition-trademark"></a>
</div>
<div>

Should be respected

</div>
</div>
hello
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn block_with_additional_classname() {
        let content = r#"
```admonish tip.my-style.other-style
Will have bonus classnames
```
"#;

        let expected = r##"

<div id="admonition-tip" class="admonition tip my-style other-style">
<div class="admonition-title">

Tip

<a class="admonition-anchor-link" href="#admonition-tip"></a>
</div>
<div>

Will have bonus classnames

</div>
</div>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn block_with_additional_classname_and_title() {
        let content = r#"
```admonish tip.my-style.other-style "Developers don't want you to know this one weird tip!"
Will have bonus classnames
```
"#;

        let expected = r##"

<div id="admonition-developers-dont-want-you-to-know-this-one-weird-tip" class="admonition tip my-style other-style">
<div class="admonition-title">

Developers don't want you to know this one weird tip!

<a class="admonition-anchor-link" href="#admonition-developers-dont-want-you-to-know-this-one-weird-tip"></a>
</div>
<div>

Will have bonus classnames

</div>
</div>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn block_with_empty_additional_classnames_title_content() {
        let content = r#"
```admonish .... ""
```
"#;

        let expected = r#"

<div id="admonition-default" class="admonition note">
<div>



</div>
</div>
"#;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn unique_ids_same_title() {
        let content = r#"
```admonish note "My Note"
Content zero.
```

```admonish note "My Note"
Content one.
```
"#;

        let expected = r##"

<div id="admonition-my-note" class="admonition note">
<div class="admonition-title">

My Note

<a class="admonition-anchor-link" href="#admonition-my-note"></a>
</div>
<div>

Content zero.

</div>
</div>


<div id="admonition-my-note-1" class="admonition note">
<div class="admonition-title">

My Note

<a class="admonition-anchor-link" href="#admonition-my-note-1"></a>
</div>
<div>

Content one.

</div>
</div>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn v2_config_works() {
        let content = r#"
```admonish tip class="my other-style" title="Article Heading"
Bonus content!
```
"#;

        let expected = r##"

<div id="admonition-article-heading" class="admonition tip my other-style">
<div class="admonition-title">

Article Heading

<a class="admonition-anchor-link" href="#admonition-article-heading"></a>
</div>
<div>

Bonus content!

</div>
</div>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn continue_on_error_output() {
        let content = r#"
```admonish title="
Bonus content!
```
"#;

        let expected = r##"

<div id="admonition-error-rendering-admonishment" class="admonition bug">
<div class="admonition-title">

Error rendering admonishment

<a class="admonition-anchor-link" href="#admonition-error-rendering-admonishment"></a>
</div>
<div>

Failed with:

```log
TOML parsing error: TOML parse error at line 1, column 8
  |
1 | title="
  |        ^
invalid basic string

```

Original markdown input:

````markdown
```admonish title="
Bonus content!
```
````


</div>
</div>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn bail_on_error_output() {
        let content = r#"
```admonish title="
Bonus content!
```
"#;
        assert_eq!(
            preprocess(
                content,
                OnFailure::Bail,
                &AdmonitionDefaults::default(),
                RenderTextMode::Html
            )
            .unwrap_err()
            .to_string(),
            r#"Error processing admonition, bailing:
```admonish title="
Bonus content!
```"#
                .to_owned()
        )
    }

    #[test]
    fn test_renderer_strip_explicit() {
        let content = r#"
````admonish title="Title"
```rust
let x = 10;
x = 20;
```
````
"#;
        assert_eq!(
            preprocess(
                content,
                OnFailure::Bail,
                &AdmonitionDefaults::default(),
                RenderTextMode::Strip
            )
            .unwrap(),
            r#"

```rust
let x = 10;
x = 20;
```

"#
            .to_owned()
        )
    }

    #[test]
    fn block_collapsible() {
        let content = r#"
```admonish collapsible=true
Hidden
```
"#;

        let expected = r##"

<details id="admonition-note" class="admonition note">
<summary class="admonition-title">

Note

<a class="admonition-anchor-link" href="#admonition-note"></a>
</summary>
<div>

Hidden

</div>
</details>
"##;

        assert_eq!(expected, prep(content));
    }

    #[test]
    fn default_toml_title() {
        let content = r#"# Chapter
```admonish
A simple admonition.
```
Text
"#;

        let expected = r##"# Chapter

<div id="admonition-admonish" class="admonition note">
<div class="admonition-title">

Admonish

<a class="admonition-anchor-link" href="#admonition-admonish"></a>
</div>
<div>

A simple admonition.

</div>
</div>
Text
"##;

        let preprocess_result = preprocess(
            content,
            OnFailure::Continue,
            &AdmonitionDefaults {
                title: Some("Admonish".to_owned()),
                collapsible: false,
            },
            RenderTextMode::Html,
        )
        .unwrap();
        assert_eq!(expected, preprocess_result);
    }

    #[test]
    fn empty_explicit_title_with_default() {
        let content = r#"# Chapter
```admonish title=""
A simple admonition.
```
Text
"#;

        let expected = r#"# Chapter

<div id="admonition-default" class="admonition note">
<div>

A simple admonition.

</div>
</div>
Text
"#;

        let preprocess_result = preprocess(
            content,
            OnFailure::Continue,
            &AdmonitionDefaults {
                title: Some("Admonish".to_owned()),
                collapsible: false,
            },
            RenderTextMode::Html,
        )
        .unwrap();
        assert_eq!(expected, preprocess_result);
    }

    #[test]
    fn empty_explicit_title() {
        let content = r#"# Chapter
```admonish title=""
A simple admonition.
```
Text
"#;

        let expected = r#"# Chapter

<div id="admonition-default" class="admonition note">
<div>

A simple admonition.

</div>
</div>
Text
"#;

        assert_eq!(expected, prep(content));
    }
}
