//! Non-destructive content-type filters. User groups are stored independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentKind {
    Text,
    Link,
    Code,
    Html,
    Image,
}

impl ContentKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Link => "link",
            Self::Code => "code",
            Self::Html => "html",
            Self::Image => "image",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "文本",
            Self::Link => "链接",
            Self::Code => "代码",
            Self::Html => "HTML",
            Self::Image => "图片",
        }
    }
}

pub fn classify(content: &str) -> ContentKind {
    let value = content.trim();
    if !value.chars().any(char::is_whitespace)
        && url::Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https" | "ftp") && url.host_str().is_some()
        })
    {
        return ContentKind::Link;
    }
    if value.contains('<')
        && (value.contains("</") || value.contains("/>") || value.contains("<!DOCTYPE"))
    {
        return ContentKind::Html;
    }
    let json = (value.starts_with('{') || value.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(value).is_ok();
    let fenced = value.starts_with("```") && value.lines().count() > 1;
    let declaration = value.lines().any(|line| {
        let line = line.trim_start();
        [
            "fn ",
            "pub fn ",
            "def ",
            "class ",
            "function ",
            "import ",
            "#include ",
            "SELECT ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
    }) && value.lines().count() > 1;
    if json || fenced || declaration {
        ContentKind::Code
    } else {
        ContentKind::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn type_filters_do_not_modify_or_mislabel_prose() {
        assert_eq!(
            classify(" https://example.org/path?q=1 "),
            ContentKind::Link
        );
        assert_eq!(classify("参考 https://example.org"), ContentKind::Text);
        assert_eq!(classify("http://"), ContentKind::Text);
        assert_eq!(classify("{会议记录}"), ContentKind::Text);
        assert_eq!(classify("{\"enabled\": true}"), ContentKind::Code);
        assert_eq!(
            classify("fn main() {\n println!(\"hello\");\n}"),
            ContentKind::Code
        );
    }
}
