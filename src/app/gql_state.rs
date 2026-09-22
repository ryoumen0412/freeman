//! GraphQL domain logic — methods that operate purely on GraphQLState.

use crate::app::state::GraphQLState;
use crate::messages::ui_events::GqlField;

impl GraphQLState {
    // ========================
    // Field navigation
    // ========================

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            GqlField::Endpoint => GqlField::Query,
            GqlField::Query => GqlField::Variables,
            GqlField::Variables => GqlField::Endpoint,
        };
        match self.active_field {
            GqlField::Endpoint => self.endpoint_cursor = self.endpoint.len(),
            GqlField::Query => self.query_cursor = self.query.len(),
            GqlField::Variables => self.variables_cursor = self.variables.len(),
        }
    }

    // ========================
    // Field editing (start)
    // ========================

    /// Prepare endpoint for editing. Caller should set input_mode.
    pub fn edit_endpoint(&mut self) {
        self.active_field = GqlField::Endpoint;
        self.endpoint_cursor = self.endpoint.len();
    }

    /// Prepare query for editing. Caller should set input_mode.
    pub fn edit_query(&mut self) {
        self.active_field = GqlField::Query;
        self.query_cursor = self.query.len();
    }

    /// Prepare variables for editing. Caller should set input_mode.
    pub fn edit_variables(&mut self) {
        self.active_field = GqlField::Variables;
        self.variables_cursor = self.variables.len();
    }

    // ========================
    // Text input
    // ========================

    pub fn char_input(&mut self, c: char) {
        match self.active_field {
            GqlField::Endpoint => {
                let cursor = self.endpoint_cursor;
                self.endpoint_cursor =
                    crate::app::text_utils::insert_char(&mut self.endpoint, cursor, c);
            }
            GqlField::Query => {
                let cursor = self.query_cursor;
                self.query_cursor = crate::app::text_utils::insert_char(&mut self.query, cursor, c);
            }
            GqlField::Variables => {
                let cursor = self.variables_cursor;
                self.variables_cursor =
                    crate::app::text_utils::insert_char(&mut self.variables, cursor, c);
            }
        }
    }

    pub fn backspace(&mut self) {
        match self.active_field {
            GqlField::Endpoint => {
                let cursor = self.endpoint_cursor;
                self.endpoint_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.endpoint, cursor);
            }
            GqlField::Query => {
                let cursor = self.query_cursor;
                self.query_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.query, cursor);
            }
            GqlField::Variables => {
                let cursor = self.variables_cursor;
                self.variables_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.variables, cursor);
            }
        }
    }

    pub fn cursor_left(&mut self) {
        match self.active_field {
            GqlField::Endpoint => {
                self.endpoint_cursor = crate::app::text_utils::prev_char_boundary(
                    &self.endpoint,
                    self.endpoint_cursor,
                );
            }
            GqlField::Query => {
                self.query_cursor =
                    crate::app::text_utils::prev_char_boundary(&self.query, self.query_cursor);
            }
            GqlField::Variables => {
                self.variables_cursor = crate::app::text_utils::prev_char_boundary(
                    &self.variables,
                    self.variables_cursor,
                );
            }
        }
    }

    pub fn cursor_right(&mut self) {
        match self.active_field {
            GqlField::Endpoint => {
                self.endpoint_cursor = crate::app::text_utils::next_char_boundary(
                    &self.endpoint,
                    self.endpoint_cursor,
                );
            }
            GqlField::Query => {
                self.query_cursor =
                    crate::app::text_utils::next_char_boundary(&self.query, self.query_cursor);
            }
            GqlField::Variables => {
                self.variables_cursor = crate::app::text_utils::next_char_boundary(
                    &self.variables,
                    self.variables_cursor,
                );
            }
        }
    }

    // ========================
    // Scrolling
    // ========================

    pub fn scroll_up(&mut self) {
        self.response_scroll = self.response_scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.response_scroll = self.response_scroll.saturating_add(1);
    }

    // ========================
    // Response handling
    // ========================

    pub fn apply_response(&mut self, id: u64, body: String, time_ms: u64) {
        if self.pending_request_id == Some(id) {
            self.response = body;
            self.time_ms = time_ms;
            self.is_loading = false;
            self.pending_request_id = None;
        }
    }

    pub fn apply_error(&mut self, id: u64, error: String, time_ms: u64) {
        if self.pending_request_id == Some(id) {
            self.response = format!("Error: {}", error);
            self.time_ms = time_ms;
            self.is_loading = false;
            self.pending_request_id = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::GraphQLState;
    use crate::messages::ui_events::GqlField;

    fn make_gql() -> GraphQLState {
        GraphQLState::default()
    }

    // ── Field navigation ─────────────────────────────────────────────────────

    #[test]
    fn test_next_field_cycles_endpoint_query_variables() {
        let mut gql = make_gql();
        gql.active_field = GqlField::Endpoint;
        gql.next_field();
        assert_eq!(gql.active_field, GqlField::Query);
        gql.next_field();
        assert_eq!(gql.active_field, GqlField::Variables);
        gql.next_field();
        assert_eq!(gql.active_field, GqlField::Endpoint);
    }

    // ── Edit start ───────────────────────────────────────────────────────────

    #[test]
    fn test_edit_endpoint_sets_field_and_cursor() {
        let mut gql = make_gql();
        gql.edit_endpoint();
        assert_eq!(gql.active_field, GqlField::Endpoint);
        assert_eq!(gql.endpoint_cursor, gql.endpoint.len());
    }

    #[test]
    fn test_edit_query_sets_field_and_cursor() {
        let mut gql = make_gql();
        gql.edit_query();
        assert_eq!(gql.active_field, GqlField::Query);
        assert_eq!(gql.query_cursor, gql.query.len());
    }

    #[test]
    fn test_edit_variables_sets_field_and_cursor() {
        let mut gql = make_gql();
        gql.edit_variables();
        assert_eq!(gql.active_field, GqlField::Variables);
        assert_eq!(gql.variables_cursor, gql.variables.len());
    }

    // ── Character input per field ────────────────────────────────────────────

    #[test]
    fn test_char_input_endpoint_modifies_endpoint() {
        let mut gql = make_gql();
        gql.active_field = GqlField::Endpoint;
        gql.endpoint_cursor = gql.endpoint.len();
        let orig_len = gql.endpoint.len();
        gql.char_input('!');
        assert_eq!(gql.endpoint.len(), orig_len + 1);
        assert!(gql.endpoint.ends_with('!'));
    }

    #[test]
    fn test_char_input_query_modifies_query() {
        let mut gql = make_gql();
        gql.active_field = GqlField::Query;
        gql.query_cursor = gql.query.len();
        let orig_len = gql.query.len();
        gql.char_input('x');
        assert_eq!(gql.query.len(), orig_len + 1);
    }

    #[test]
    fn test_char_input_variables_modifies_variables() {
        let mut gql = make_gql();
        gql.active_field = GqlField::Variables;
        gql.variables_cursor = gql.variables.len();
        let orig_len = gql.variables.len();
        gql.char_input('z');
        assert_eq!(gql.variables.len(), orig_len + 1);
    }

    // ── Backspace per field ──────────────────────────────────────────────────

    #[test]
    fn test_backspace_endpoint_removes_char() {
        let mut gql = make_gql();
        gql.active_field = GqlField::Endpoint;
        gql.endpoint = "http://test".to_string();
        gql.endpoint_cursor = gql.endpoint.len();
        gql.backspace();
        assert_eq!(gql.endpoint, "http://tes");
    }

    // ── Scrolling ────────────────────────────────────────────────────────────

    #[test]
    fn test_scroll_up_does_not_underflow() {
        let mut gql = make_gql();
        gql.response_scroll = 0;
        gql.scroll_up();
        assert_eq!(gql.response_scroll, 0);
    }

    #[test]
    fn test_scroll_down_increments() {
        let mut gql = make_gql();
        gql.scroll_down();
        assert_eq!(gql.response_scroll, 1);
    }

    // ── Response handling ────────────────────────────────────────────────────

    #[test]
    fn test_apply_response_updates_state() {
        let mut gql = make_gql();
        gql.pending_request_id = Some(1);
        gql.is_loading = true;
        gql.apply_response(1, r#"{"data":{}}"#.to_string(), 50);
        assert_eq!(gql.response, r#"{"data":{}}"#);
        assert_eq!(gql.time_ms, 50);
        assert!(!gql.is_loading);
        assert!(gql.pending_request_id.is_none());
    }

    #[test]
    fn test_apply_response_wrong_id_ignored() {
        let mut gql = make_gql();
        gql.pending_request_id = Some(1);
        gql.is_loading = true;
        gql.apply_response(99, "ignored".to_string(), 10);
        assert!(gql.is_loading); // unchanged
    }

    #[test]
    fn test_apply_error_updates_state() {
        let mut gql = make_gql();
        gql.pending_request_id = Some(1);
        gql.is_loading = true;
        gql.apply_error(1, "timeout".to_string(), 5000);
        assert!(gql.response.contains("Error: timeout"));
        assert!(!gql.is_loading);
    }
}
