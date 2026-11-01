//! Finds the messages in a snapshot of Discord's accessibility tree. Pure, so it is tested anywhere
//! against a tree written the way the Discord report prints it.
//!
//! What Discord shows screen readers (checked against a real report, November 2026):
//!   ListItem id="chat-messages-<channel>-<message>"
//!     Group                                    the message ("article")
//!       Text  name="Alex Server tag 06:53"     the header: author, tag and a time with id "message-timestamp-<message>"
//!       Text  name="Hello"                     the message text, one Text per paragraph, no id
//!       Group name="Message actions"           the hover toolbar, laid over the header
//! Follow-up messages from the same author have no header. A reply's quote sits above the header.

/// One element of the tree, as read through UI Automation.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub kind: String, // UI Automation control type: "Text", "Group", "ListItem", …
    pub id: String,
    pub name: String,
    pub rect: (i32, i32, i32, i32), // left, top, right, bottom in screen pixels
    pub children: Vec<Node>,
}

/// A message on screen: its text blocks (paragraphs, headings, list items), each translated and drawn
/// over its own place, and the whole message row (which Discord lights up under the mouse). Embeds,
/// pictures, spoilers and reactions are not text blocks: they stay as Discord draws them.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: String,
    pub parts: Vec<Part>,
    pub row: (i32, i32, i32, i32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub text: String,
    pub rect: (i32, i32, i32, i32),
}

impl Message {
    /// All of its text, one block per line: what is translated, and the cache key.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Text without custom emoji names (":wave:"), which screen readers get instead of the picture.
fn without_emoji_names(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(':') {
        let after = &rest[start + 1..];
        let name_len = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '~'));
        match name_len {
            Some(n) if n >= 2 && after[n..].starts_with(':') => {
                out.push_str(&rest[..start]);
                rest = &after[n + 1..];
            }
            _ => {
                out.push_str(&rest[..=start]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Node {
    fn any(&self, f: &dyn Fn(&Node) -> bool) -> bool {
        f(self) || self.children.iter().any(|c| c.any(f))
    }

    fn visible(&self) -> bool {
        self.rect.2 > self.rect.0 && self.rect.3 > self.rect.1
    }

    /// Its own name, or the names of the pieces under it run together (Chromium splits a line into
    /// runs that carry their own spaces).
    fn text(&self) -> String {
        if !self.name.trim().is_empty() || self.children.is_empty() {
            return self.name.trim().to_string();
        }
        self.children
            .iter()
            .map(Node::text)
            .collect::<String>()
            .trim()
            .to_string()
    }
}

/// The messages in the tree, and the path (child indexes from `root`) to the smallest element that
/// holds them all: the message list, re-read on its own afterwards because it is much smaller.
pub fn messages(root: &Node) -> (Vec<Message>, Option<Vec<usize>>) {
    let mut found = Vec::new();
    let mut common: Option<Vec<usize>> = None;
    walk(root, &mut Vec::new(), &mut |node, path| {
        let Some(rest) = node.id.strip_prefix("chat-messages-") else {
            return true;
        };
        let id = rest.rsplit('-').next().unwrap_or(rest).to_string();
        if let Some(message) = message(node, id, viewport(root, path)) {
            found.push(message);
        }
        // The list's own parent: the scroller, whose rectangle is the visible part of the list.
        let parent = &path[..path.len().saturating_sub(2)];
        common = Some(match common.take() {
            None => parent.to_vec(),
            Some(c) => c
                .iter()
                .zip(parent)
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect(),
        });
        false // nothing worth reading inside a message's own subtree
    });
    (found, common)
}

/// The part of the screen where an element can be seen: the overlap of all the elements around it.
/// A long message scrolled half out of view has its hidden blocks above or below this.
fn viewport(root: &Node, path: &[usize]) -> (i32, i32, i32, i32) {
    let mut clip = (i32::MIN, i32::MIN, i32::MAX, i32::MAX);
    let mut node = root;
    for &i in path {
        if node.visible() {
            clip = (
                clip.0.max(node.rect.0),
                clip.1.max(node.rect.1),
                clip.2.min(node.rect.2),
                clip.3.min(node.rect.3),
            );
        }
        node = &node.children[i];
    }
    clip
}

fn walk(node: &Node, path: &mut Vec<usize>, visit: &mut dyn FnMut(&Node, &[usize]) -> bool) {
    if !visit(node, path) {
        return;
    }
    for (i, child) in node.children.iter().enumerate() {
        path.push(i);
        walk(child, path, visit);
        path.pop();
    }
}

fn is_time(n: &Node) -> bool {
    n.id.starts_with("message-timestamp-")
}

/// A line's text with any timestamp in it left out.
fn without_time(n: &Node) -> String {
    if is_time(n) {
        String::new()
    } else if !n.any(&is_time) {
        n.text()
    } else {
        n.children
            .iter()
            .map(without_time)
            .collect::<String>()
            .trim()
            .to_string()
    }
}

/// Where a line's text is, leaving out any timestamp in it.
fn rect_without_time(n: &Node) -> Option<(i32, i32, i32, i32)> {
    if is_time(n) || !n.visible() {
        return None;
    }
    if !n.any(&is_time) {
        return Some(n.rect);
    }
    n.children
        .iter()
        .filter(|c| !without_time(c).is_empty())
        .filter_map(rect_without_time)
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// The text of one message item, or None when it has none (an image, a sticker, a system line).
fn message(item: &Node, id: String, clip: (i32, i32, i32, i32)) -> Option<Message> {
    let article = item
        .children
        .iter()
        .find(|c| c.kind == "Group")
        .unwrap_or(item);
    // The header holds the time and the author's name (a button); a follow-up message has neither.
    let header = article
        .children
        .iter()
        .find(|c| c.any(&is_time) && c.any(&|n: &Node| n.kind == "Button"));
    let below = header.map_or(article.rect.1, |h| h.rect.3);

    let blocks: Vec<&Node> = article
        .children
        .iter()
        .filter(|c| {
            c.visible() && !std::ptr::eq(*c, header.map_or(std::ptr::null(), |h| h as *const Node))
        })
        .collect();
    // From a block's left edge to the message's right edge, or to whatever sits beside it on the same
    // line (a link after "Try it:"): room for a longer translation without covering anything else.
    let room = |r: (i32, i32, i32, i32)| {
        blocks
            .iter()
            .map(|b| b.rect)
            .filter(|o| *o != r && o.0 >= r.2 - 2 && o.1 < r.3 - 2 && o.3 > r.1 + 2)
            .map(|o| o.0 - 2)
            .fold(article.rect.2 - 16, i32::min)
            .max(r.2)
    };
    let part = |n: &Node| {
        // A follow-up message can carry its (hover-only) time inside its first line: left out.
        let text = without_emoji_names(&without_time(n));
        // Screen readers get ", " between pieces; a block never ends in one.
        let text = text
            .trim_end_matches(|c: char| c == ',' || c.is_whitespace())
            .to_string();
        let rect = rect_without_time(n)?;
        // Only blocks wholly on screen: a half-hidden one would be drawn over Discord's own bars.
        let shown = rect.1 >= clip.1
            && rect.3 <= clip.3
            && rect.0 >= clip.0
            && rect.2 <= clip.2.saturating_add(16);
        (shown && !text.is_empty()).then(|| Part {
            text,
            rect: (rect.0, rect.1, room(rect), rect.3),
        })
    };
    let only_text = |n: &Node| {
        !n.any(&|d: &Node| matches!(d.kind.as_str(), "Button" | "Image" | "Hyperlink" | "List"))
    };
    let mut parts = Vec::new();
    for c in blocks.iter().filter(|c| c.rect.1 >= below - 4) {
        match c.kind.as_str() {
            // The label beside a spoiler's button only names the button.
            "Text"
                if blocks
                    .iter()
                    .any(|b| b.kind == "Button" && b.name == c.name) => {}
            "Text" => parts.extend(part(c)),
            // A bulleted or numbered list: one block per item.
            "List" => parts.extend(
                c.children
                    .iter()
                    .filter(|i| i.kind == "ListItem")
                    .filter_map(part),
            ),
            // A quote: plain text in a box. Embeds and attachments have links, buttons and pictures.
            "Group" if only_text(c) => parts.extend(part(c)),
            _ => {}
        }
    }
    (!parts.is_empty()).then_some(Message {
        id,
        parts,
        row: article.rect,
    })
}

/// Whether a translation kept (nearly) all the words of the original, ignoring case and punctuation.
pub fn same_words(a: &str, b: &str) -> bool {
    let words = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| {
                w.chars()
                    .flat_map(char::to_lowercase)
                    .filter(|c| *c != '\u{307}')
                    .collect()
            })
            .collect()
    };
    let (a, mut b) = (words(a), words(b));
    let longest = a.len().max(b.len());
    if longest == 0 {
        return true;
    }
    let kept = a
        .iter()
        .filter(|w| {
            b.iter()
                .position(|x| x == *w)
                .map(|i| b.swap_remove(i))
                .is_some()
        })
        .count();
    kept * 4 >= longest * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a tree from the Discord report's format: two spaces per level,
    /// `Some(Kind) id="…" name="…" rect=Some((l, t, r, b))`.
    fn parse(report: &str) -> Node {
        let mut stack: Vec<(usize, Node)> = Vec::new();
        let mut root = None;
        for line in report
            .lines()
            .filter(|l| l.trim_start().starts_with("Some("))
        {
            let depth = (line.len() - line.trim_start().len()) / 2;
            let line = line.trim_start();
            let kind = line["Some(".len()..line.find(')').unwrap()].to_string();
            let quoted = |key: &str| {
                let start = line.find(key).unwrap() + key.len();
                let end = start + line[start..].find("\" ").unwrap();
                line[start..end].to_string()
            };
            let nums: Vec<i32> = line[line.find("rect=Some((").unwrap() + 11..]
                .trim_end_matches("))")
                .split(", ")
                .map(|n| n.parse().unwrap())
                .collect();
            let node = Node {
                kind,
                id: quoted("id=\""),
                name: quoted("name=\""),
                rect: (nums[0], nums[1], nums[2], nums[3]),
                children: Vec::new(),
            };
            while stack.last().is_some_and(|(d, _)| *d >= depth) {
                let (_, done) = stack.pop().unwrap();
                match stack.last_mut() {
                    Some((_, parent)) => parent.children.push(done),
                    None => root = Some(done),
                }
            }
            stack.push((depth, node));
        }
        while let Some((_, done)) = stack.pop() {
            match stack.last_mut() {
                Some((_, parent)) => parent.children.push(done),
                None => root = Some(done),
            }
        }
        root.unwrap()
    }

    // The shape of a real report, names changed. The second message is a follow-up of the first (no
    // header); the third is an image with no text; the hover toolbar is open on the first.
    const REPORT: &str = r##"
Some(Window) id="" name="#general | Server - Discord" rect=Some((-8, -8, 1928, 1040))
  Some(Pane) id="" name="" rect=Some((0, 0, 1920, 1032))
    Some(Document) id="RootWebArea" name="#general | Server" rect=Some((0, 0, 1920, 1032))
      Some(Group) id="" name="Server sidebar" rect=Some((0, 32, 72, 982))
        Some(Text) id="" name="Direct Messages" rect=Some((35, 48, 140, 70))
      Some(Group) id="" name="general (channel)" rect=Some((340, 82, 1655, 1032))
        Some(List) id="" name="Messages in general" rect=Some((340, 630, 1639, 1010))
          Some(Text) id="" name="Welcome to #general!" rect=Some((356, 786, 498, 807))
          Some(ListItem) id="chat-messages-15545-15662" name="AlexServer tag: MDLN06:53Hello there" rect=Some((340, 894, 1639, 942))
            Some(Group) id="" name="Alex Server tag: MDLN , Hello there , 06:53" rect=Some((340, 894, 1639, 942))
              Some(Text) id="" name="Alex Server tag: MDLN 06:53" rect=Some((412, 896, 1615, 918))
                Some(Button) id="" name="Alex" rect=Some((412, 896, 445, 917))
                Some(Group) id="" name="" rect=Some((514, 900, 544, 916))
                  Some(Text) id="message-timestamp-15662" name="" rect=Some((514, 900, 544, 916))
              Some(Text) id="" name="Hello there" rect=Some((412, 918, 498, 939))
              Some(Group) id="" name="Message actions" rect=Some((1382, 878, 1639, 912))
                Some(Text) id="" name=":heart:" rect=Some((1415, 894, 1455, 913))
                Some(Text) id="" name="Click to react" rect=Some((1415, 912, 1528, 929))
          Some(ListItem) id="chat-messages-15545-15663" name="Wie geht's?" rect=Some((340, 942, 1639, 966))
            Some(Group) id="" name="Wie geht's?" rect=Some((340, 942, 1639, 966))
              Some(Group) id="" name="" rect=Some((356, 946, 400, 962))
                Some(Text) id="message-timestamp-15663" name="" rect=Some((356, 946, 400, 962))
              Some(Text) id="" name="" rect=Some((412, 943, 520, 964))
                Some(Text) id="" name="Wie geht's" rect=Some((412, 943, 500, 964))
                Some(Text) id="" name="?" rect=Some((500, 943, 506, 964))
          Some(ListItem) id="chat-messages-15545-15665" name="06:54 Und dir" rect=Some((340, 966, 1639, 990))
            Some(Group) id="" name="Und dir" rect=Some((340, 966, 1639, 990))
              Some(Text) id="" name="" rect=Some((356, 967, 1600, 989))
                Some(Group) id="" name="" rect=Some((356, 970, 400, 986))
                  Some(Text) id="message-timestamp-15665" name="" rect=Some((356, 970, 400, 986))
                Some(Text) id="" name="Und dir" rect=Some((412, 967, 470, 989))
          Some(ListItem) id="chat-messages-15545-15664" name="image.png" rect=Some((340, 990, 1639, 1006))
            Some(Group) id="" name="" rect=Some((340, 990, 1639, 1006))
              Some(Image) id="" name="image.png" rect=Some((412, 990, 600, 1006))
      Some(Edit) id="" name="Message #general" rect=Some((412, 967, 1442, 1023))
"##;

    fn part(text: &str, rect: (i32, i32, i32, i32)) -> Part {
        Part {
            text: text.into(),
            rect,
        }
    }

    #[test]
    fn rich_message_is_split_into_blocks() {
        let report = r##"
Some(ListItem) id="chat-messages-1-2" name="" rect=Some((340, 100, 1639, 600))
  Some(Group) id="" name="" rect=Some((340, 100, 1639, 600))
    Some(Text) id="" name="sleax 22:07" rect=Some((412, 102, 1615, 124))
      Some(Button) id="" name="sleax" rect=Some((412, 102, 450, 124))
      Some(Group) id="" name="" rect=Some((460, 104, 500, 120))
        Some(Text) id="message-timestamp-2" name="" rect=Some((460, 104, 500, 120))
    Some(Text) id="" name="Stalkly Beta is live :eyes:" rect=Some((412, 126, 700, 160))
      Some(Image) id="" name=":eyes:" rect=Some((660, 130, 690, 156))
    Some(Text) id="" name="What's inside" rect=Some((412, 180, 520, 200))
    Some(List) id="" name="" rect=Some((430, 200, 1000, 260))
      Some(ListItem) id="" name="Voice time by channel" rect=Some((430, 200, 1000, 230))
      Some(ListItem) id="" name="Smart insights" rect=Some((430, 230, 1000, 260))
    Some(Group) id="" name="Stalkly: Discord analytics" rect=Some((412, 400, 900, 500))
      Some(Button) id="" name="Remove all embeds" rect=Some((880, 400, 900, 420))
      Some(Text) id="" name="Stalkly: Discord analytics" rect=Some((420, 410, 800, 430))
    Some(Image) id="" name="video" rect=Some((412, 510, 900, 590))
"##;
        let (found, _) = messages(&parse(report));
        assert_eq!(
            found[0].parts,
            [
                part("Stalkly Beta is live", (412, 126, 1623, 160)),
                part("What's inside", (412, 180, 1623, 200)),
                part("Voice time by channel", (430, 200, 1623, 230)),
                part("Smart insights", (430, 230, 1623, 260)),
            ]
        );
        assert_eq!(without_emoji_names("a :xy: b 10:30 c:"), "a b 10:30 c:");
    }

    #[test]
    fn finds_messages_in_a_real_tree() {
        let (found, list) = messages(&parse(REPORT));
        assert_eq!(
            found,
            [
                Message {
                    id: "15662".into(),
                    parts: vec![part("Hello there", (412, 918, 1623, 939))],
                    row: (340, 894, 1639, 942)
                },
                Message {
                    id: "15663".into(),
                    parts: vec![part("Wie geht's?", (412, 943, 1623, 964))],
                    row: (340, 942, 1639, 966)
                },
                // A follow-up whose time sits inside its line.
                Message {
                    id: "15665".into(),
                    parts: vec![part("Und dir", (412, 967, 1623, 989))],
                    row: (340, 966, 1639, 990)
                },
            ]
        );
        // Window > Pane > Document > channel group (index 1) > the list.
        assert_eq!(list, Some(vec![0, 0, 1]));
    }

    /// A real report: `QT_DISCORD_REPORT=path cargo test real_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_report() {
        let path = std::env::var("QT_DISCORD_REPORT").expect("QT_DISCORD_REPORT");
        let (found, list) = messages(&parse(&std::fs::read_to_string(path).unwrap()));
        println!("list {list:?}");
        for m in &found {
            println!("{m:?}");
        }
        assert!(!found.is_empty());
    }

    #[test]
    fn same_words_ignores_case_and_punctuation() {
        assert!(same_words("Naber knk, nasılsın?", "naber knk nasılsın"));
        assert!(same_words("İyi akşamlar", "iyi akşamlar!"));
        assert!(!same_words("How are you?", "Nasılsın?"));
        assert!(!same_words("I am at home now", "Ben şimdi evdeyim"));
    }

    #[test]
    fn nothing_without_messages() {
        let (found, list) = messages(&parse(
            r#"Some(Window) id="" name="Discord" rect=Some((0, 0, 10, 10))"#,
        ));
        assert!(found.is_empty() && list.is_none());
    }
}
