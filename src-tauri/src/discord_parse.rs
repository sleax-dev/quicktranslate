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

/// A message on screen: its text and where the text is.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: String,
    pub text: String,
    pub rect: (i32, i32, i32, i32),
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
        if let Some(message) = message(node, id) {
            found.push(message);
        }
        let parent = &path[..path.len().saturating_sub(1)];
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

/// The text of one message item, or None when it has none (an image, a sticker, a system line).
fn message(item: &Node, id: String) -> Option<Message> {
    let article = item
        .children
        .iter()
        .find(|c| c.kind == "Group")
        .unwrap_or(item);
    let is_time = |n: &Node| n.id.starts_with("message-timestamp-");
    // The header holds the time and the author's name (a button); a follow-up message has neither.
    let header = article
        .children
        .iter()
        .find(|c| c.any(&is_time) && c.any(&|n: &Node| n.kind == "Button"));
    let below = header.map_or(article.rect.1, |h| h.rect.3);

    let paragraphs: Vec<&Node> = article
        .children
        .iter()
        .filter(|c| c.kind == "Text" && c.visible() && c.rect.1 >= below - 4)
        .filter(|c| !c.any(&is_time))
        .filter(|c| !std::ptr::eq(*c, header.map_or(std::ptr::null(), |h| h as *const Node)))
        .collect();
    let text = paragraphs
        .iter()
        .map(|p| p.text())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        return None;
    }
    // From the text's left edge to the message's right edge: room for a longer translation, and the
    // colour sampled there is the message's background, not the text.
    let left = paragraphs.iter().map(|p| p.rect.0).min()?;
    let top = paragraphs.iter().map(|p| p.rect.1).min()?;
    let bottom = paragraphs.iter().map(|p| p.rect.3).max()?;
    let right = (article.rect.2 - 16).max(left + 1);
    Some(Message {
        id,
        text,
        rect: (left, top, right, bottom),
    })
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
        Some(List) id="" name="Messages in general" rect=Some((340, 630, 1639, 982))
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
          Some(ListItem) id="chat-messages-15545-15664" name="image.png" rect=Some((340, 966, 1639, 982))
            Some(Group) id="" name="" rect=Some((340, 966, 1639, 982))
              Some(Image) id="" name="image.png" rect=Some((412, 966, 600, 982))
      Some(Edit) id="" name="Message #general" rect=Some((412, 967, 1442, 1023))
"##;

    #[test]
    fn finds_messages_in_a_real_tree() {
        let (found, list) = messages(&parse(REPORT));
        assert_eq!(
            found,
            [
                Message {
                    id: "15662".into(),
                    text: "Hello there".into(),
                    rect: (412, 918, 1623, 939)
                },
                Message {
                    id: "15663".into(),
                    text: "Wie geht's?".into(),
                    rect: (412, 943, 1623, 964)
                },
            ]
        );
        // Window > Pane > Document > channel group (index 1) > the list.
        assert_eq!(list, Some(vec![0, 0, 1, 0]));
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
    fn nothing_without_messages() {
        let (found, list) = messages(&parse(
            r#"Some(Window) id="" name="Discord" rect=Some((0, 0, 10, 10))"#,
        ));
        assert!(found.is_empty() && list.is_none());
    }
}
