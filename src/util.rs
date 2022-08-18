use std::collections::HashMap;
use std::ops::Range;

pub struct OwnedCaptures {
    text: String,
    named_groups: HashMap<String, Range<usize>>,
    captures: Vec<Range<usize>>,
}

impl OwnedCaptures {
    pub fn get(&self, i: usize) -> Option<&str> {
        let range = self.captures.get(i)?;
        Some(&self.text[range.start..range.end])
    }

    pub fn name(&self, name: &str) -> Option<&str> {
        let range = self.named_groups.get(name)?;
        Some(&self.text[range.start..range.end])
    }
}

pub trait FancyRegexExt {
    fn owned_captures(&self, text: &str) -> fancy_regex::Result<Option<OwnedCaptures>>;
}

impl FancyRegexExt for fancy_regex::Regex {
    fn owned_captures(&self, text: &str) -> fancy_regex::Result<Option<OwnedCaptures>> {
        let (named_groups, captures) = if let Some(c) = self.captures(text)? {
            let named_groups: HashMap<String, Range<_>> = self
                .capture_names()
                .flatten()
                .filter_map(|g| c.name(g).map(|m| (g.to_string(), m.range())))
                .collect();
            let captures: Vec<Range<_>> = c.iter().flatten().map(|m| m.range()).collect();
            (named_groups, captures)
        } else {
            return Ok(None);
        };
        Ok(Some(OwnedCaptures {
            text: text.to_string(),
            named_groups,
            captures,
        }))
    }
}
