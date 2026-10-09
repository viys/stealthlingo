//! Converts Wiktionary's HTML into plain terminal text.

/// Strips HTML tags, decodes common entities and collapses whitespace.
pub fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    collapse_whitespace(&decode_entities(&text))
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let decoded = after.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &after[..end];
            let ch = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                    .and_then(char::from_u32),
            };
            ch.map(|c| (c, end))
        });
        match decoded {
            Some((ch, end)) => {
                out.push(ch);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_and_decodes_entities() {
        assert_eq!(
            html_to_text(
                r#"Lasting for a <a href="/wiki/short" title="short">short</a> period&nbsp;of &amp; <b>time</b>&#33; &#x41;"#
            ),
            "Lasting for a short period of & time! A"
        );
        assert_eq!(html_to_text("<span class=\"x\"></span>   "), "");
        assert_eq!(html_to_text("AT&T & co"), "AT&T & co");
    }
}
