//! A mod's description as rich text. Authors paste what they wrote for
//! the Workshop (BBCode) or use the game's own tags (`<b>`, `<color=…>`).

/// BBCode tags with a plain HTML counterpart.
const PAIRS: [(&str, &str); 12] = [
    ("b", "b"),
    ("i", "i"),
    ("u", "u"),
    ("strike", "s"),
    ("h1", "h3"),
    ("h2", "h4"),
    ("h3", "h4"),
    ("list", "ul"),
    ("olist", "ol"),
    ("quote", "blockquote"),
    ("code", "pre"),
    ("spoiler", "i"),
];

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The game's tags as BBCode where there is one; the others dropped.
fn without_game_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out += &rest[..open];
        let inside = &rest[open + 1..];
        let named = inside.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/');
        let Some(close) = inside.find('>').filter(|_| named) else {
            out.push('<');
            rest = inside;
            continue;
        };
        let tag = inside[..close].to_lowercase();
        if matches!(tag.as_str(), "b" | "/b" | "i" | "/i") {
            out += &format!("[{tag}]");
        }
        rest = &inside[close + 1..];
    }
    out + rest
}

fn safe_link(url: &str) -> bool {
    ["https://", "http://", "steam://"]
        .iter()
        .any(|s| url.starts_with(s))
}

/// The HTML for one tag, or `None` for a tag that is not markup. A link
/// is closed only if it was opened: one to an address that is not a
/// page never is.
fn tag_html(tag: &str, in_link: &mut bool) -> Option<String> {
    let (closing, name) = match tag.strip_prefix('/') {
        Some(name) => (true, name),
        None => (false, tag),
    };
    let (name, value) = name.split_once('=').unwrap_or((name, ""));
    let name = name.to_lowercase();
    if let Some((_, html)) = PAIRS.iter().find(|(bb, _)| *bb == name) {
        return Some(format!("<{}{html}>", if closing { "/" } else { "" }));
    }
    Some(match (name.as_str(), closing) {
        ("*", false) => "<li>".to_string(),
        ("hr", false) => "<hr>".to_string(),
        ("url", false) if safe_link(value) => {
            *in_link = true;
            format!("<a href=\"{value}\">")
        }
        ("url", true) if std::mem::take(in_link) => "</a>".to_string(),
        ("hr" | "table" | "tr" | "td" | "th" | "noparse" | "url", _) => String::new(),
        _ => return None,
    })
}

/// `text` as the subset of HTML a Qt text view shows.
pub fn rich(text: &str) -> String {
    let text = escape(&without_game_tags(text));
    let mut out = String::with_capacity(text.len() + 64);
    let mut rest = text.as_str();
    let mut in_link = false;
    while let Some(open) = rest.find('[') {
        out += &rest[..open];
        rest = &rest[open..];
        let Some(close) = rest.find(']') else {
            break;
        };
        let tag = &rest[1..close];
        let after = &rest[close + 1..];
        if tag.eq_ignore_ascii_case("img") {
            // Pictures are left out, with the address between the tags.
            rest = after.find("[/img]").map_or("", |end| &after[end + 6..]);
        } else if tag.eq_ignore_ascii_case("url") {
            // No address given: the text between the tags is the address.
            let end = after.find("[/url]").unwrap_or(after.len());
            let url = &after[..end];
            out += &match safe_link(url) {
                true => format!("<a href=\"{url}\">{url}</a>"),
                false => url.to_string(),
            };
            rest = after.get(end + 6..).unwrap_or("");
        } else if let Some(html) = tag_html(tag, &mut in_link) {
            out += &html;
            rest = after;
        } else {
            out.push('[');
            rest = &rest[1..];
        }
    }
    out += rest;
    // A line break beside a block is the block's own.
    let mut html = out.replace("\r\n", "\n");
    for block in [
        "<ul>",
        "</ul>",
        "<ol>",
        "</ol>",
        "<li>",
        "<hr>",
        "</h3>",
        "</h4>",
        "<blockquote>",
        "</blockquote>",
    ] {
        html = html
            .replace(&format!("\n{block}"), block)
            .replace(&format!("{block}\n"), block);
    }
    html.trim().replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_markup_becomes_html() {
        assert_eq!(
            rich("[h1]Title[/h1]\n[b]Bold[/b] and [i]slanted[/i]\nNext"),
            "<h3>Title</h3><b>Bold</b> and <i>slanted</i><br>Next"
        );
        assert_eq!(
            rich("[list]\n[*]One\n[*][b]Two[/b]\n[/list]\n[hr][/hr]"),
            "<ul><li>One<li><b>Two</b></ul><hr>"
        );
        assert_eq!(rich("[quote]Said[/quote]"), "<blockquote>Said</blockquote>");
    }

    #[test]
    fn links_open_only_pages() {
        assert_eq!(
            rich("[url=https://example.org/?a=1&b=2]Page[/url]"),
            "<a href=\"https://example.org/?a=1&amp;b=2\">Page</a>"
        );
        assert_eq!(
            rich("[url]https://example.org[/url]"),
            "<a href=\"https://example.org\">https://example.org</a>"
        );
        assert_eq!(rich("[url=javascript:x]Bad[/url]"), "Bad");
        assert_eq!(rich("[url]file:///etc[/url]"), "file:///etc");
    }

    #[test]
    fn pictures_are_left_out_and_other_brackets_stay() {
        assert_eq!(rich("[img]REPLACE_HERO[/img]\nText"), "Text");
        assert_eq!(rich("[CF] Mod [1.6] a[0]"), "[CF] Mod [1.6] a[0]");
        assert_eq!(rich("open [ never closed"), "open [ never closed");
    }

    #[test]
    fn the_game_s_own_tags_are_read_and_html_is_not() {
        assert_eq!(
            rich("<b>Bold</b> <color=#ff0000>red</color>"),
            "<b>Bold</b> red"
        );
        assert_eq!(rich("a < b & c"), "a &lt; b &amp; c");
        assert_eq!(rich("x [script]y"), "x [script]y");
    }
}
