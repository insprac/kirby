use std::collections::HashMap;

/// Represents a robots.txt file for a website, currently supports allow/disallow rules
/// (including wildcards) and sitemaps.
#[derive(Debug, Clone)]
pub struct RobotsTxt<'a> {
    /// Mapping of user-agent -> rule.
    rules: HashMap<&'a str, RobotsTxtRule<'a>>,
    /// A list of sitemaps if any were included in the robots.txt file.
    sitemaps: Vec<&'a str>,
    /// A list of all agents sorted by length for faster matching.
    agents_ordered: Vec<&'a str>,
}

impl<'a> RobotsTxt<'a> {
    /// Parse a raw robots.txt file, this can not fail since any incorrectly formatted lines or
    /// unsupported directives are simply ignored.
    ///
    /// The file input must live as long as the created RobotsTxt.
    ///
    /// Directives are case insensitive so they will always match (when valid and supported).
    ///
    /// # Example
    ///
    /// ```
    /// let robotstxt_file = r#"
    /// # Prevent everyone from crawling anything
    /// User-agent: *
    /// Disallow: /
    ///
    /// # Allow KirbyBot to crawl everything but /prevented/
    /// User-agent: KirbyBot*
    /// Allow: /
    /// Disallow: /prevented/
    /// "#;
    ///
    /// let robotstxt = kirby_core::robotstxt::RobotsTxt::parse(robotstxt_file);
    /// println!("{robotstxt:?}");
    /// ```
    pub fn parse(file: &'a str) -> Self {
        let mut current_agent: Option<&'a str> = None;
        let mut rules: HashMap<&'a str, RobotsTxtRule> = HashMap::new();
        let mut sitemaps: Vec<&'a str> = Vec::new();

        for line in file.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            if let Some(agent) = strip_prefix(line, "user-agent: ") {
                current_agent = Some(agent.trim());
            } else if let Some(allow) = strip_prefix(line, "allow: ") {
                if let Some(agent) = current_agent {
                    let allow = allow.trim();
                    if allow.is_empty() {
                        continue;
                    }

                    rules
                        .entry(agent)
                        .or_insert(Default::default())
                        .allow
                        .push(allow.trim());
                }
            } else if let Some(disallow) = strip_prefix(line, "disallow: ") {
                if let Some(agent) = current_agent {
                    let disallow = disallow.trim();
                    if disallow.is_empty() {
                        continue;
                    }

                    rules
                        .entry(agent)
                        .or_insert(Default::default())
                        .disallow
                        .push(disallow.trim());
                }
            } else if let Some(sitemap) = strip_prefix(line, "sitemap: ") {
                let sitemap = sitemap.trim();
                if sitemap.is_empty() {
                    continue;
                }

                sitemaps.push(sitemap)
            }
        }

        // Get agents and sort them by longest to shortest.
        let mut agents_ordered = rules.keys().map(|&a| a).collect::<Vec<&str>>();
        agents_ordered.sort_by(|a, b| b.len().cmp(&a.len()));

        // Sort all rule allow and disallow by longest to shortest
        rules.iter_mut().for_each(|(_, rule)| {
            rule.allow.sort_by(|a, b| b.len().cmp(&a.len()));
            rule.disallow.sort_by(|a, b| b.len().cmp(&a.len()));
        });

        Self {
            rules,
            sitemaps,
            agents_ordered,
        }
    }

    pub fn is_allowed(&self, user_agent: &str, path: &str) -> bool {
        let Some(rules) = self.get_agent_rules(user_agent) else {
            return true;
        };

        rules.is_allowed(path)
    }

    fn find_matching_agent(&self, user_agent: &str) -> Option<&str> {
        self.agents_ordered
            .iter()
            .find(|&&pattern| match_pattern(pattern, user_agent))
            .map(|&a| a)
    }

    fn get_agent_rules(&self, user_agent: &str) -> Option<&RobotsTxtRule> {
        self
            .find_matching_agent(user_agent)
            // Unwrapping is safe here because we know rules must contain the pattern returned from
            // `self.find_matching_agent` is guaranteed to be a key.
            .map(|pattern| self.rules.get(pattern).unwrap())
    }
}

#[derive(Debug, Clone, Default)]
struct RobotsTxtRule<'a> {
    allow: Vec<&'a str>,
    disallow: Vec<&'a str>,
}

impl<'a> RobotsTxtRule<'a> {
    /// Checks if a path is allowed for this rule, if there is are multiple allows and/or disallows
    /// it will choose the most matching (longest length of the pattern).
    ///
    /// If no allow or disallow matches then the path is allowed.
    fn is_allowed(&self, path: &str) -> bool {
        let best_allow = self.allow.iter().find(|&&pattern| match_pattern(pattern, path));
        let best_disallow = self.disallow.iter().find(|&&pattern| match_pattern(pattern, path));
        match (best_allow, best_disallow) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(allow), Some(disallow)) => allow.len() > disallow.len(),
            (None, None) => true,
        }
    }
}

/// Strips prefix from a &str ignoring the case and returning the rest of the text.
fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }

    if s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Matches wildcard patterns where * matches everything in between including '/' characters.
/// If no wildcards are present it will simply match the start of the string.
fn match_pattern(pattern: &str, string: &str) -> bool {
    if !pattern.contains("*") && string.starts_with(pattern) {
        return true;
    }

    fn match_recursive(p: &[char], s: &[char]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some('*'), _) => {
                match_recursive(&p[1..], s) || (!s.is_empty() && match_recursive(p, &s[1..]))
            }
            (Some(pc), Some(sc)) if pc == sc => match_recursive(&p[1..], &s[1..]),
            _ => false,
        }
    }

    let pattern_chars: Vec<char> = pattern.chars().collect();
    let string_chars: Vec<char> = string.chars().collect();
    match_recursive(&pattern_chars, &string_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_well_formatted_robotstxt() {
        let robotstxt_file = r#"
        # Prevent everyone from crawling anything
        User-agent: *
        Disallow: /
        
        # Allow KirbyBot to crawl everything but /prevented/
        User-agent: KirbyBot
        Allow: /
        Disallow: /prevented/

        Sitemap: https://www.example.com/sitemap.xml
        "#;

        let robotstxt = RobotsTxt::parse(robotstxt_file);

        assert_eq!(robotstxt.rules.keys().count(), 2);
        assert!(robotstxt.rules.contains_key("*"));
        assert!(robotstxt.rules.contains_key("KirbyBot"));

        let wildcard_rules = robotstxt.rules.get("*").unwrap();
        assert_eq!(wildcard_rules.allow, Vec::<String>::new());
        assert_eq!(wildcard_rules.disallow, vec!["/"]);

        let kirby_rules = robotstxt.rules.get("KirbyBot").unwrap();
        assert_eq!(kirby_rules.allow, vec!["/"]);
        assert_eq!(kirby_rules.disallow, vec!["/prevented/"]);

        assert_eq!(
            robotstxt.sitemaps,
            vec!["https://www.example.com/sitemap.xml"]
        );

        assert_eq!(robotstxt.agents_ordered, vec!["KirbyBot", "*"]);
    }

    #[test]
    fn parse_badly_formatted_robotstxt() {
        let robotstxt_file = r#"
        This is just wrong
        // Definitely not a robots.txt comment...
        # The allow is ignored because no user-agent is provided yet
        Allow: /allowed
        # The sitemap will work as expected
        Sitemap: https://www.example.com/sitemap.xml

        # Mixing cases is allowed
        user-agent: Kirby
        ALLOW: /
        DisALLow: /
        Allow: /something
        "#;

        let robotstxt = RobotsTxt::parse(robotstxt_file);

        let user_agents = robotstxt.rules.keys().map(|&a| a).collect::<Vec<&str>>();
        assert_eq!(user_agents, vec!["Kirby"]);

        let kirby_rules = robotstxt.rules.get("Kirby").unwrap();
        assert_eq!(kirby_rules.allow, vec!["/", "/something"]);
        assert_eq!(kirby_rules.disallow, vec!["/"]);

        assert_eq!(
            robotstxt.sitemaps,
            vec!["https://www.example.com/sitemap.xml"]
        );

        assert_eq!(robotstxt.agents_ordered, vec!["Kirby"]);
    }

    #[test]
    fn matches_patterns_correctly() {
        let pattern = "/test/*.txt";
        assert!(match_pattern(pattern, "/test/path/file.txt"));
        assert!(!match_pattern(pattern, "/test/path/file.png"));

        let pattern = "/test/*/something.html";
        assert!(match_pattern(
            pattern,
            "/test/some/long/path/something.html"
        ));
        assert!(!match_pattern(
            pattern,
            "/test/some/long/pathsomething.html"
        ));

        let pattern = "/";
        assert!(match_pattern(pattern, "/test/files/index.html"));
        assert!(match_pattern(pattern, "/"));
        assert!(!match_pattern(pattern, "test"));

        let pattern = "*.html";
        assert!(match_pattern(pattern, "/test/files/index.html"));
        assert!(match_pattern(pattern, "test.html"));
        assert!(!match_pattern(pattern, "/"));

        let pattern = "/test/*/middle/prefix*/file.txt";
        assert!(match_pattern(
            pattern,
            "/test/in/the/middle/prefixstillmatches/ok/file.txt"
        ));
        assert!(match_pattern(pattern, "/test/in/middle/prefix/file.txt"));
        assert!(!match_pattern(pattern, "/test/middle/prefix/file.txt"));
    }

    #[test]
    fn find_matching_agent() {
        let robotstxt_file = r#"
        User-agent: T*
        Allow: /

        User-agent: KirbyBot/*
        Allow: /

        User-agent: Kirby*
        Allow: /

        User-agent: GoogleBot
        Disallow: /
        "#;

        let robotstxt = RobotsTxt::parse(robotstxt_file);
        assert_eq!(robotstxt.find_matching_agent("Kirby"), Some("Kirby*"));
        assert_eq!(robotstxt.find_matching_agent("KirbyBot"), Some("Kirby*"));
        assert_eq!(
            robotstxt.find_matching_agent("KirbyBot/1.0"),
            Some("KirbyBot/*")
        );
        assert_eq!(
            robotstxt.find_matching_agent("GoogleBot"),
            Some("GoogleBot")
        );
        assert_eq!(
            robotstxt.find_matching_agent("GoogleBot/1.0"),
            Some("GoogleBot")
        );
        assert_eq!(robotstxt.find_matching_agent("SomethingElse"), None);
    }
}
