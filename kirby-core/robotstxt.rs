use std::collections::HashMap;

/// Represents a robots.txt file for a website, currently supports allow/disallow rules (including
/// wildcards) and sitemaps.
#[derive(Debug, Clone)]
pub struct RobotsTxt {
    rules: HashMap<String, RobotsTxtRule>,
    sitemaps: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RobotsTxtRule {
    allow: Vec<String>,
    disallow: Vec<String>,
}

impl RobotsTxt {
    /// Parse a raw robots.txt file, this can not fail since any incorrectly formatted lines or
    /// unsupported directives are simply ignored.
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
    pub fn parse(file: &str) -> Self {
        let mut current_agent: Option<&str> = None;
        let mut rules: HashMap<String, RobotsTxtRule> = HashMap::new();
        let mut sitemaps: Vec<String> = Vec::new();

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
                        .entry(agent.to_string())
                        .or_insert(Default::default())
                        .allow
                        .push(allow.trim().to_string());
                }
            } else if let Some(disallow) = strip_prefix(line, "disallow: ") {
                if let Some(agent) = current_agent {
                    let disallow = disallow.trim();
                    if disallow.is_empty() {
                        continue;
                    }

                    rules
                        .entry(agent.to_string())
                        .or_insert(Default::default())
                        .disallow
                        .push(disallow.trim().to_string());
                }
            } else if let Some(sitemap) = strip_prefix(line, "sitemap: ") {
                let sitemap = sitemap.trim();
                if sitemap.is_empty() {
                    continue;
                }

                sitemaps.push(sitemap.to_string())
            }
        }

        Self { rules, sitemaps }
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
        assert_eq!(wildcard_rules.disallow, vec!["/".to_string()]);

        let kirby_rules = robotstxt.rules.get("KirbyBot").unwrap();
        assert_eq!(kirby_rules.allow, vec!["/".to_string()]);
        assert_eq!(kirby_rules.disallow, vec!["/prevented/".to_string()]);

        assert_eq!(robotstxt.sitemaps, vec!["https://www.example.com/sitemap.xml".to_string()])
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

        let user_agents = robotstxt.rules.keys().collect::<Vec<&String>>();
        assert_eq!(user_agents, vec!["Kirby"]);

        let kirby_rules = robotstxt.rules.get("Kirby").unwrap();
        assert_eq!(kirby_rules.allow, vec!["/".to_string(), "/something".to_string()]);
        assert_eq!(kirby_rules.disallow, vec!["/".to_string()]);

        assert_eq!(robotstxt.sitemaps, vec!["https://www.example.com/sitemap.xml".to_string()])
    }
}
