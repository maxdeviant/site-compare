use anyhow::Result;
use auk::renderer::HtmlElementRenderer;
use auk::*;
use pulldown_cmark::{self as md};
use similar::{ChangeTag, TextDiff};

use crate::{Comparison, Difference};

pub fn render_report(comparison: Comparison) -> Result<String> {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (path, difference) in comparison.differences {
        match difference {
            Difference::Added => added.push(path),
            Difference::Changed { before, after } => changed.push((path, before, after)),
            Difference::Removed => removed.push(path),
        }
    }

    let css = r#"
        .diff-remove {
            display: block;
            width: 100%;
            background-color: #ffcccc;
        }

        .diff-add {
            display: block;
            width: 100%;
            background-color: #ccffcc;
        }
    "#;

    let report_html =
        html()
            .lang("en")
            .child(
                head()
                    .child(meta().charset("utf-8"))
                    .child(meta().http_equiv("x-ua-compatible").content("ie=edge"))
                    .child(
                        meta()
                            .name("viewport")
                            .content("width=device-width, initial-scale=1.0, maximum-scale=1"),
                    )
                    .child(title().child("Site Comparison"))
                    .child(style().child(css)),
            )
            .child(
                body()
                    .child(h1().child("Comparison Report"))
                    .child(div().child(h2().child("Identical files")).child(
                        ol().children(comparison.identical.iter().map(|path| li().child(path))),
                    ))
                    .child(
                        div()
                            .child(h2().child("Added files"))
                            .child(ol().children(added.into_iter().map(|path| li().child(path)))),
                    )
                    .child(
                        div()
                            .child(h2().child("Removed files"))
                            .child(ol().children(removed.into_iter().map(|path| li().child(path)))),
                    )
                    .child(
                        div()
                            .child(h2().child("Changed files"))
                            .child(ol().children(changed.into_iter().map(
                                |(path, before, after)| {
                                    li().child(path).child(changed_file_diff(&before, &after))
                                },
                            ))),
                    ),
            );

    Ok(HtmlElementRenderer::new().render_to_string(&report_html)?)
}

fn changed_file_diff(before: &str, after: &str) -> HtmlElement {
    let diff = TextDiff::from_lines(before, after);

    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let (sign, class) = match change.tag() {
            ChangeTag::Delete => ("-", Some("diff-remove")),
            ChangeTag::Insert => ("+", Some("diff-add")),
            ChangeTag::Equal => (" ", None),
        };

        lines.push(
            span()
                .class::<&str>(class)
                .child(escape_html(&format!("{sign}{change}"))),
        )
    }

    pre().child(code().children(lines))
}

fn escape_html(text: &str) -> String {
    let mut escaped_text = String::with_capacity(text.len());
    md::escape::escape_html(&mut escaped_text, &text).unwrap();
    escaped_text
}
