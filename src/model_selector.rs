//! Model selector overlay state.
//!
//! This is used by the interactive TUI to present a searchable list of models.

use crate::models::ModelEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelKey {
    pub provider: String,
    pub id: String,
}

impl ModelKey {
    #[must_use]
    pub fn full_id(&self) -> String {
        format!("{}/{}", self.provider, self.id)
    }
}

#[derive(Debug)]
pub struct ModelSelectorOverlay {
    all: Vec<ModelKey>,
    filtered: Vec<usize>,
    selected: usize,
    query: String,
    max_visible: usize,
}

impl ModelSelectorOverlay {
    #[must_use]
    pub fn new(models: &[ModelEntry]) -> Self {
        let keys = models
            .iter()
            .map(|entry| ModelKey {
                provider: entry.model.provider.clone(),
                id: entry.model.id.clone(),
            })
            .collect::<Vec<_>>();
        Self::new_from_keys(keys)
    }

    #[must_use]
    pub fn new_from_keys(mut keys: Vec<ModelKey>) -> Self {
        keys.sort_by(|a, b| a.provider.cmp(&b.provider).then_with(|| a.id.cmp(&b.id)));
        let mut selector = Self {
            all: keys,
            filtered: Vec::new(),
            selected: 0,
            query: String::new(),
            max_visible: 10,
        };
        selector.refresh_filtered();
        selector
    }

    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    #[must_use]
    pub const fn max_visible(&self) -> usize {
        self.max_visible
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.max_visible = max_visible.max(1);
    }

    pub fn clear_query(&mut self) {
        if self.query.is_empty() {
            return;
        }
        self.query.clear();
        self.refresh_filtered();
    }

    pub fn push_chars<I: IntoIterator<Item = char>>(&mut self, chars: I) {
        let mut changed = false;
        for ch in chars {
            if ch.is_control() {
                continue;
            }
            self.query.push(ch);
            changed = true;
        }
        if changed {
            self.refresh_filtered();
        }
    }

    pub fn pop_char(&mut self) {
        if self.query.pop().is_some() {
            self.refresh_filtered();
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.filtered.len() - 1);
        }
    }

    pub fn select_page_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let step = self.max_visible.saturating_sub(1).max(1);
        self.selected = (self.selected + step).min(self.filtered.len() - 1);
    }

    pub fn select_page_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let step = self.max_visible.saturating_sub(1).max(1);
        self.selected = self.selected.saturating_sub(step);
    }

    #[must_use]
    pub fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    #[must_use]
    pub fn item_at(&self, filtered_index: usize) -> Option<&ModelKey> {
        self.filtered
            .get(filtered_index)
            .and_then(|&idx| self.all.get(idx))
    }

    #[must_use]
    pub fn selected_item(&self) -> Option<&ModelKey> {
        self.item_at(self.selected)
    }

    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    #[must_use]
    pub const fn scroll_offset(&self) -> usize {
        if self.selected < self.max_visible {
            0
        } else {
            self.selected - self.max_visible + 1
        }
    }

    fn refresh_filtered(&mut self) {
        self.filtered = self
            .all
            .iter()
            .enumerate()
            .filter_map(|(idx, key)| matches_query(&self.query, key).then_some(idx))
            .collect();
        self.selected = 0;
    }
}

fn matches_query(query: &str, key: &ModelKey) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }

    fuzzy_match(trimmed, &key.full_id())
        || fuzzy_match(trimmed, &key.provider)
        || fuzzy_match(trimmed, &key.id)
}

fn fuzzy_match(pattern: &str, value: &str) -> bool {
    let needle_str = pattern.to_lowercase();
    let haystack_str = value.to_lowercase();
    let mut needle = needle_str.chars().filter(|c| !c.is_whitespace());
    let mut haystack = haystack_str.chars();
    for ch in needle.by_ref() {
        if !haystack.by_ref().any(|h| h == ch) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selector(keys: &[(&str, &str)]) -> ModelSelectorOverlay {
        ModelSelectorOverlay::new_from_keys(
            keys.iter()
                .map(|(provider, id)| ModelKey {
                    provider: (*provider).to_string(),
                    id: (*id).to_string(),
                })
                .collect(),
        )
    }

    #[test]
    fn filters_with_fuzzy_subsequence() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("anthropic", "claude-sonnet-4")]);
        selector.push_chars("og".chars());
        assert_eq!(selector.filtered_len(), 1);
        assert_eq!(selector.selected_item().unwrap().full_id(), "openai/gpt-4o");
    }

    #[test]
    fn filters_case_insensitive_and_whitespace_insensitive() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("openai", "gpt-4o-mini")]);
        selector.push_chars("GPT 4O".chars());
        assert_eq!(selector.filtered_len(), 2);
    }

    #[test]
    fn selection_wraps() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("openai", "gpt-4o-mini")]);
        selector.select_prev();
        assert_eq!(
            selector.selected_item().unwrap().full_id(),
            "openai/gpt-4o-mini"
        );
        selector.select_next();
        assert_eq!(selector.selected_item().unwrap().full_id(), "openai/gpt-4o");
    }

    #[test]
    fn new_from_keys_sorts_provider_then_id() {
        let selector = selector(&[
            ("openai", "gpt-4o-mini"),
            ("anthropic", "claude-sonnet-4"),
            ("openai", "gpt-4o"),
        ]);
        let ids = (0..selector.filtered_len())
            .map(|idx| selector.item_at(idx).unwrap().full_id())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "anthropic/claude-sonnet-4",
                "openai/gpt-4o",
                "openai/gpt-4o-mini"
            ]
        );
    }

    #[test]
    fn page_navigation_respects_window_and_bounds() {
        let mut selector = selector(&[
            ("openai", "a"),
            ("openai", "b"),
            ("openai", "c"),
            ("openai", "d"),
            ("openai", "e"),
        ]);
        selector.set_max_visible(3);
        assert_eq!(selector.max_visible(), 3);
        assert_eq!(selector.selected_index(), 0);
        assert_eq!(selector.scroll_offset(), 0);

        selector.select_page_down();
        assert_eq!(selector.selected_index(), 2);
        assert_eq!(selector.scroll_offset(), 0);

        selector.select_page_down();
        assert_eq!(selector.selected_index(), 4);
        assert_eq!(selector.scroll_offset(), 2);

        selector.select_page_down();
        assert_eq!(selector.selected_index(), 4);

        selector.select_page_up();
        assert_eq!(selector.selected_index(), 2);
        assert_eq!(selector.scroll_offset(), 0);

        selector.select_page_up();
        assert_eq!(selector.selected_index(), 0);
    }

    #[test]
    fn set_max_visible_clamps_to_one() {
        let mut selector = selector(&[("openai", "a"), ("openai", "b"), ("openai", "c")]);
        selector.set_max_visible(0);
        assert_eq!(selector.max_visible(), 1);

        selector.select_page_down();
        assert_eq!(selector.selected_index(), 1);
        selector.select_page_down();
        assert_eq!(selector.selected_index(), 2);
    }

    #[test]
    fn query_input_ignores_control_chars_and_pop_refreshes() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("openai", "o1")]);
        selector.push_chars("o1\n\t".chars());
        assert_eq!(selector.query(), "o1");
        assert_eq!(selector.filtered_len(), 1);
        assert_eq!(selector.selected_item().unwrap().full_id(), "openai/o1");

        selector.pop_char();
        assert_eq!(selector.query(), "o");
        assert_eq!(selector.filtered_len(), 2);
    }

    #[test]
    fn clear_query_noop_when_empty_and_reset_when_non_empty() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("openai", "o1")]);

        selector.select_next();
        assert_eq!(selector.selected_index(), 1);
        selector.clear_query();
        assert_eq!(selector.selected_index(), 1);

        selector.push_chars("1".chars());
        assert_eq!(selector.filtered_len(), 1);
        selector.clear_query();
        assert_eq!(selector.query(), "");
        assert_eq!(selector.filtered_len(), 2);
        assert_eq!(selector.selected_index(), 0);
    }

    #[test]
    fn no_match_has_no_selected_item_and_navigation_is_stable() {
        let mut selector = selector(&[("openai", "gpt-4o"), ("openai", "o1")]);
        selector.push_chars("zzz".chars());

        assert_eq!(selector.filtered_len(), 0);
        assert!(selector.selected_item().is_none());
        assert!(selector.item_at(0).is_none());

        selector.select_next();
        selector.select_prev();
        selector.select_page_down();
        selector.select_page_up();

        assert_eq!(selector.selected_index(), 0);
        assert_eq!(selector.scroll_offset(), 0);
    }
}
