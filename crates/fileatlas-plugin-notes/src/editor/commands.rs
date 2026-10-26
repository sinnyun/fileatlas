use std::ops::Range;

pub fn wrap_selection(
    text: &str,
    selection: Range<usize>,
    prefix: &str,
    suffix: &str,
    placeholder: &str,
) -> (String, Range<usize>) {
    let selected = &text[selection.clone()];
    let content = if selected.is_empty() {
        placeholder
    } else {
        selected
    };
    let replacement = format!("{prefix}{content}{suffix}");
    let inner = selection.start + prefix.len()..selection.start + prefix.len() + content.len();
    let mut output = text.to_owned();
    output.replace_range(selection, &replacement);
    (output, inner)
}

pub fn prefix_lines(text: &str, selection: Range<usize>, prefix: &str) -> (String, Range<usize>) {
    let start = text[..selection.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let end = text[selection.end..]
        .find('\n')
        .map_or(text.len(), |offset| selection.end + offset);
    let block = &text[start..end];
    let lines = block.split('\n').collect::<Vec<_>>();
    let remove = lines.iter().all(|line| line.starts_with(prefix));
    let replaced = lines
        .iter()
        .map(|line| {
            if remove {
                line.strip_prefix(prefix).unwrap_or(line).to_owned()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let next = start..start + replaced.len();
    let mut output = text.to_owned();
    output.replace_range(start..end, &replaced);
    (output, next)
}

pub fn toggle_task_lines(text: &str, selection: Range<usize>) -> (String, Range<usize>) {
    let start = text[..selection.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let end = text[selection.end..]
        .find('\n')
        .map_or(text.len(), |offset| selection.end + offset);
    let block = &text[start..end];
    let replaced = block
        .split('\n')
        .map(|line| {
            if let Some(rest) = line.strip_prefix("- [ ] ") {
                format!("- [x] {rest}")
            } else if let Some(rest) = line
                .strip_prefix("- [x] ")
                .or_else(|| line.strip_prefix("- [X] "))
            {
                format!("- [ ] {rest}")
            } else {
                format!("- [ ] {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let next = start..start + replaced.len();
    let mut output = text.to_owned();
    output.replace_range(start..end, &replaced);
    (output, next)
}

pub fn replace_all(text: &str, query: &str, replacement: &str) -> (String, usize) {
    if query.is_empty() {
        return (text.to_owned(), 0);
    }
    let count = text.match_indices(query).count();
    (text.replace(query, replacement), count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_format_respects_selection_and_inserts_placeholder_at_caret() {
        let (selected, range) = wrap_selection("你好 world", 7..12, "**", "**", "文字");
        assert_eq!(selected, "你好 **world**");
        assert_eq!(&selected[range], "world");

        let (inserted, range) = wrap_selection("abc", 1..1, "[", "](url)", "链接文字");
        assert_eq!(inserted, "a[链接文字](url)bc");
        assert_eq!(&inserted[range], "链接文字");
    }

    #[test]
    fn line_prefix_toggles_all_selected_lines_and_keeps_following_lines() {
        let (listed, _) = prefix_lines("before\none\ntwo\nafter", 9..12, "- ");
        assert_eq!(listed, "before\n- one\n- two\nafter");
        let (plain, _) = prefix_lines(&listed, 10..16, "- ");
        assert_eq!(plain, "before\none\ntwo\nafter");
    }

    #[test]
    fn replace_all_counts_matches_and_ignores_empty_queries() {
        assert_eq!(replace_all("a a", "a", "b"), ("b b".into(), 2));
        assert_eq!(replace_all("abc", "", "x"), ("abc".into(), 0));
    }

    #[test]
    fn task_toggle_marks_and_completes_selected_lines() {
        let (checked, _) = toggle_task_lines("one\n- [ ] two", 0..3);
        assert_eq!(checked, "- [ ] one\n- [ ] two");
        let (done, _) = toggle_task_lines(&checked, 0..checked.len());
        assert_eq!(done, "- [x] one\n- [x] two");
        let (pending, _) = toggle_task_lines(&done, 0..done.len());
        assert_eq!(pending, checked);
    }
}
