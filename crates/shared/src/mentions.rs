use crate::models::{Mention, MentionKind};

/// Parse mentions from message content.
/// Syntax: <@123> for users, <@&456> for roles
pub fn parse(content: &str) -> Vec<Mention> {
    let mut mentions = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' && i + 2 < len && bytes[i + 1] == b'@' {
            let start = i;
            i += 2; // skip '<@'
            let kind = if i < len && bytes[i] == b'&' {
                i += 1; // skip '&'
                MentionKind::Role
            } else {
                MentionKind::User
            };
            // Parse digits
            let digit_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > digit_start && i < len && bytes[i] == b'>' {
                if let Ok(id) = content[digit_start..i].parse::<i64>() {
                    mentions.push(Mention { kind, id });
                }
                i += 1; // skip '>'
            } else {
                i = start + 1; // backtrack
            }
        } else {
            i += 1;
        }
    }

    mentions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user_mention() {
        let mentions = parse("Hello <@42>!");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].id, 42);
        assert!(matches!(mentions[0].kind, MentionKind::User));
    }

    #[test]
    fn parse_role_mention() {
        let mentions = parse("Hey <@&3>!");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].id, 3);
        assert!(matches!(mentions[0].kind, MentionKind::Role));
    }

    #[test]
    fn parse_multiple() {
        let mentions = parse("<@1> and <@&2> and <@3>");
        assert_eq!(mentions.len(), 3);
    }

    #[test]
    fn parse_no_mentions() {
        let mentions = parse("just text");
        assert!(mentions.is_empty());
    }
}
