use anyhow::Result;
use auk::renderer::HtmlElementRenderer;
use auk::*;
use pulldown_cmark::{self as md};
use similar::{ChangeTag, TextDiff};
use slug::slugify;

use crate::{Comparison, Difference};

struct ChangedFile {
    path: String,
    lines_added: i32,
    lines_removed: i32,
    diff_lines: Vec<HtmlElement>,
}

pub fn render_report(comparison: Comparison) -> Result<String> {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    let mut total_lines_added = 0;
    let mut total_lines_removed = 0;

    for (path, difference) in comparison.differences {
        match difference {
            Difference::Added => added.push(path),
            Difference::Changed { before, after } => {
                let diff = TextDiff::from_lines(&before, &after);

                let mut lines_added = 0;
                let mut lines_removed = 0;
                let mut lines = Vec::new();

                for change in diff.iter_all_changes() {
                    let (sign, class) = match change.tag() {
                        ChangeTag::Insert => {
                            lines_added += 1;
                            ("+", Some("diff-line diff-add"))
                        }
                        ChangeTag::Delete => {
                            lines_removed += 1;
                            ("-", Some("diff-line diff-remove"))
                        }
                        ChangeTag::Equal => (" ", None),
                    };

                    lines.push(
                        span()
                            .class::<&str>(class)
                            .child(escape_html(&format!("{sign}{change}"))),
                    )
                }

                total_lines_added += lines_added;
                total_lines_removed += lines_removed;

                changed.push(ChangedFile {
                    path,
                    lines_added,
                    lines_removed,
                    diff_lines: lines,
                })
            }
            Difference::Removed => removed.push(path),
        }
    }

    let css = r#"
        .diff-line {
            display: block;
            width: 100%;
        }

        .diff-add {
            background-color: #ccffcc;
        }

        .diff-remove {
            background-color: #ffcccc;
        }
    "#;

    let report_html = html()
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
                .child(
                    div().child(h2().child("Identical files")).child(
                        ol().children(comparison.identical.iter().map(|path| li().child(path))),
                    ),
                )
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
                        .child(
                            h2().child("Changed files")
                                .child(diff_indicator(total_lines_added, total_lines_removed)),
                        )
                        .child(ol().children(changed.iter().map(|file| {
                            li().child(&file.path)
                                .child(
                                    a().href(format!("#diff-{}", slugify(&file.path)))
                                        .child("diff"),
                                )
                                .child(diff_indicator(file.lines_added, file.lines_removed))
                        }))),
                )
                .child(
                    div()
                        .child(h2().child("Diffs"))
                        .children(changed.into_iter().map(|file| {
                            div()
                                .id(format!("diff-{}", slugify(&file.path)))
                                .child(file.path)
                                .child(
                                    div()
                                        .child(diff_indicator(file.lines_added, file.lines_removed))
                                        .child(pre().child(code().children(file.diff_lines))),
                                )
                        })),
                ),
        );

    Ok(HtmlElementRenderer::new().render_to_string(&report_html)?)
}

fn diff_indicator(lines_added: i32, lines_removed: i32) -> HtmlElement {
    span()
        .child(span().class("diff-add").child(format!("+{lines_added}")))
        .child(span().child("&nbsp;"))
        .child(
            span()
                .class("diff-remove")
                .child(format!("-{lines_removed}")),
        )
}

fn escape_html(text: &str) -> String {
    let mut escaped_text = String::with_capacity(text.len());
    md::escape::escape_html(&mut escaped_text, &text).unwrap();
    escaped_text
}
