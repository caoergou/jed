use std::collections::HashSet;

use crate::engine::JsonValue;
use crate::tui::tree::{TreeLine, flatten};

pub struct TreeLineCache {
    pub lines: Vec<TreeLine>,
    doc: JsonValue,
    expanded: HashSet<String>,
}

impl TreeLineCache {
    pub fn new(doc: JsonValue, expanded: HashSet<String>) -> Self {
        let lines = flatten(&doc, &expanded);
        Self {
            lines,
            doc,
            expanded,
        }
    }

    pub fn rebuild(&mut self, doc: JsonValue, expanded: HashSet<String>) {
        self.doc = doc;
        self.expanded = expanded;
        self.lines = flatten(&self.doc, &self.expanded);
    }
}

#[cfg(test)]
mod tests {
    use super::TreeLineCache;
    use crate::engine::JsonValue;
    use indexmap::IndexMap;
    use std::collections::HashSet;

    fn make_doc() -> JsonValue {
        let mut root = IndexMap::new();
        root.insert("name".into(), JsonValue::String("Alice".into()));
        root.insert(
            "tags".into(),
            JsonValue::Array(vec![JsonValue::String("rust".into())]),
        );
        JsonValue::Object(root)
    }

    #[test]
    fn test_tree_line_cache() {
        let doc = make_doc();
        let expanded = HashSet::from_iter([String::from(".")]);

        let cache = TreeLineCache::new(doc, expanded);
        assert!(!cache.lines.is_empty());
    }
}
