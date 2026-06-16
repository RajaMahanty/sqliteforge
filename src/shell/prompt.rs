use reedline::Prompt;
use std::borrow::Cow;

/// Custom prompt for SQLiteForge
pub struct SqlPrompt {
    pub db_name: String,
}

impl SqlPrompt {
    pub fn new(db_name: &str) -> Self {
        Self {
            db_name: db_name.to_string(),
        }
    }
}

impl Prompt for SqlPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{}> ", self.db_name))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        let pad = self.db_name.len().saturating_sub(3);
        Cow::Owned(format!("{}...> ", " ".repeat(pad)))
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: reedline::PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("(search) ")
    }
}
