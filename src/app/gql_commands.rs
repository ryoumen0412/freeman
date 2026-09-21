use crate::app::AppState;
use crate::messages::ui_events::InputMode;
use crate::messages::NetworkCommand;

impl AppState {
    // ========================
    // GraphQL commands
    // ========================

    /// Execute a GraphQL query
    pub fn gql_execute_query(&mut self) -> Option<NetworkCommand> {
        if self.gql.is_loading {
            return None;
        }

        // Validate endpoint
        if let Err(error) = self.validate_url(&self.gql.endpoint) {
            self.gql.response = format!("Invalid endpoint: {}", error);
            return None;
        }

        self.gql.is_loading = true;
        self.gql.response = String::from("Executing query...");

        let id = self.next_id();
        self.gql.pending_request_id = Some(id);

        // Parse variables if not empty
        let variables = if self.gql.variables.trim().is_empty() || self.gql.variables.trim() == "{}"
        {
            None
        } else {
            Some(self.gql.variables.clone())
        };

        Some(NetworkCommand::ExecuteGraphQL {
            id,
            endpoint: self.gql.endpoint.clone(),
            query: self.gql.query.clone(),
            variables,
            headers: self.http.request.headers.clone(),
            auth: self.http.request.auth.clone(),
        })
    }

    /// Handle GraphQL response
    #[allow(dead_code)] // Reserved for expanded response handling
    pub fn handle_gql_response(&mut self, id: u64, _status: u16, body: String, time_ms: u64) {
        if self.gql.pending_request_id == Some(id) {
            self.gql.response = body;
            self.gql.time_ms = time_ms;
            self.gql.is_loading = false;
            self.gql.pending_request_id = None;
        }
    }

    /// Handle GraphQL error
    #[allow(dead_code)] // Reserved for expanded error handling
    pub fn handle_gql_error(&mut self, id: u64, error: String, time_ms: u64) {
        if self.gql.pending_request_id == Some(id) {
            self.gql.response = format!("Error: {}", error);
            self.gql.time_ms = time_ms;
            self.gql.is_loading = false;
            self.gql.pending_request_id = None;
        }
    }

    /// Start editing GraphQL endpoint
    pub fn gql_edit_endpoint(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = GqlField::Endpoint;
        self.gql.endpoint_cursor = self.gql.endpoint.len();
        self.ui.input_mode = InputMode::Editing;
    }

    /// Start editing GraphQL query
    pub fn gql_edit_query(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = GqlField::Query;
        self.gql.query_cursor = self.gql.query.len();
        self.ui.input_mode = InputMode::Editing;
    }

    /// Start editing GraphQL variables
    pub fn gql_edit_variables(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = GqlField::Variables;
        self.gql.variables_cursor = self.gql.variables.len();
        self.ui.input_mode = InputMode::Editing;
    }

    /// Cycle to next GraphQL field
    pub fn gql_next_field(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = match self.gql.active_field {
            GqlField::Endpoint => GqlField::Query,
            GqlField::Query => GqlField::Variables,
            GqlField::Variables => GqlField::Endpoint,
        };
        // Update cursor position for new field
        match self.gql.active_field {
            GqlField::Endpoint => self.gql.endpoint_cursor = self.gql.endpoint.len(),
            GqlField::Query => self.gql.query_cursor = self.gql.query.len(),
            GqlField::Variables => self.gql.variables_cursor = self.gql.variables.len(),
        }
    }

    /// Insert character into active GraphQL field
    pub fn gql_char(&mut self, c: char) {
        use crate::messages::ui_events::GqlField;
        match self.gql.active_field {
            GqlField::Endpoint => {
                let cursor = self.gql.endpoint_cursor;
                self.gql.endpoint_cursor =
                    crate::app::text_utils::insert_char(&mut self.gql.endpoint, cursor, c);
            }
            GqlField::Query => {
                let cursor = self.gql.query_cursor;
                self.gql.query_cursor =
                    crate::app::text_utils::insert_char(&mut self.gql.query, cursor, c);
            }
            GqlField::Variables => {
                let cursor = self.gql.variables_cursor;
                self.gql.variables_cursor =
                    crate::app::text_utils::insert_char(&mut self.gql.variables, cursor, c);
            }
        }
    }

    /// Delete character from active GraphQL field
    pub fn gql_backspace(&mut self) {
        use crate::messages::ui_events::GqlField;
        match self.gql.active_field {
            GqlField::Endpoint => {
                let cursor = self.gql.endpoint_cursor;
                self.gql.endpoint_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.gql.endpoint, cursor);
            }
            GqlField::Query => {
                let cursor = self.gql.query_cursor;
                self.gql.query_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.gql.query, cursor);
            }
            GqlField::Variables => {
                let cursor = self.gql.variables_cursor;
                self.gql.variables_cursor =
                    crate::app::text_utils::delete_char_before(&mut self.gql.variables, cursor);
            }
        }
    }

    /// Move cursor left in active GraphQL field
    pub fn gql_cursor_left(&mut self) {
        use crate::messages::ui_events::GqlField;
        match self.gql.active_field {
            GqlField::Endpoint => {
                self.gql.endpoint_cursor = crate::app::text_utils::prev_char_boundary(
                    &self.gql.endpoint,
                    self.gql.endpoint_cursor,
                );
            }
            GqlField::Query => {
                self.gql.query_cursor = crate::app::text_utils::prev_char_boundary(
                    &self.gql.query,
                    self.gql.query_cursor,
                );
            }
            GqlField::Variables => {
                self.gql.variables_cursor = crate::app::text_utils::prev_char_boundary(
                    &self.gql.variables,
                    self.gql.variables_cursor,
                );
            }
        }
    }

    /// Move cursor right in active GraphQL field
    pub fn gql_cursor_right(&mut self) {
        use crate::messages::ui_events::GqlField;
        match self.gql.active_field {
            GqlField::Endpoint => {
                self.gql.endpoint_cursor = crate::app::text_utils::next_char_boundary(
                    &self.gql.endpoint,
                    self.gql.endpoint_cursor,
                );
            }
            GqlField::Query => {
                self.gql.query_cursor = crate::app::text_utils::next_char_boundary(
                    &self.gql.query,
                    self.gql.query_cursor,
                );
            }
            GqlField::Variables => {
                self.gql.variables_cursor = crate::app::text_utils::next_char_boundary(
                    &self.gql.variables,
                    self.gql.variables_cursor,
                );
            }
        }
    }

    /// Scroll GraphQL response up
    pub fn gql_scroll_up(&mut self) {
        self.gql.response_scroll = self.gql.response_scroll.saturating_sub(1);
    }

    /// Scroll GraphQL response down
    pub fn gql_scroll_down(&mut self) {
        self.gql.response_scroll = self.gql.response_scroll.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::messages::ui_events::{GqlField, InputMode};

    fn make_state() -> AppState {
        AppState::new()
    }

    // ── Execute query ─────────────────────────────────────────────────────────

    #[test]
    fn test_gql_execute_blocked_when_loading() {
        let mut state = make_state();
        state.gql.is_loading = true;
        let cmd = state.gql_execute_query();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_gql_execute_invalid_endpoint_returns_none_with_error() {
        let mut state = make_state();
        state.gql.endpoint = String::new();
        let cmd = state.gql_execute_query();
        assert!(cmd.is_none());
        assert!(
            state.gql.response.contains("Invalid"),
            "Expected error message, got: {}",
            state.gql.response
        );
    }

    #[test]
    fn test_gql_execute_valid_produces_command() {
        let mut state = make_state();
        state.gql.endpoint = "https://api.example.com/graphql".to_string();
        state.gql.query = "query { users { id } }".to_string();
        let cmd = state.gql_execute_query();
        assert!(cmd.is_some());
        assert!(state.gql.is_loading);
        assert!(state.gql.pending_request_id.is_some());
    }

    #[test]
    fn test_gql_execute_empty_variables_sends_none() {
        let mut state = make_state();
        state.gql.endpoint = "https://api.example.com/graphql".to_string();
        state.gql.variables = "{}".to_string();
        let cmd = state.gql_execute_query();
        if let Some(crate::messages::NetworkCommand::ExecuteGraphQL { variables, .. }) = cmd {
            assert!(variables.is_none(), "empty {{}} should send None variables");
        }
    }

    #[test]
    fn test_gql_execute_non_empty_variables_sends_some() {
        let mut state = make_state();
        state.gql.endpoint = "https://api.example.com/graphql".to_string();
        state.gql.variables = r#"{"userId": 42}"#.to_string();
        let cmd = state.gql_execute_query();
        if let Some(crate::messages::NetworkCommand::ExecuteGraphQL { variables, .. }) = cmd {
            assert!(variables.is_some());
        }
    }

    // ── Field cycling ─────────────────────────────────────────────────────────

    #[test]
    fn test_gql_next_field_cycles_endpoint_query_variables() {
        let mut state = make_state();
        // Default active_field is Query (see GraphQLState::default)
        assert_eq!(state.gql.active_field, GqlField::Query);
        state.gql_next_field();
        assert_eq!(state.gql.active_field, GqlField::Variables);
        state.gql_next_field();
        assert_eq!(state.gql.active_field, GqlField::Endpoint);
        state.gql_next_field();
        // Full cycle: back to Query
        assert_eq!(state.gql.active_field, GqlField::Query);
    }

    // ── Edit mode entry ───────────────────────────────────────────────────────

    #[test]
    fn test_gql_edit_endpoint_sets_field_and_cursor() {
        let mut state = make_state();
        state.gql.endpoint = "https://api.example.com/graphql".to_string();
        state.gql_edit_endpoint();
        assert_eq!(state.gql.active_field, GqlField::Endpoint);
        assert_eq!(state.gql.endpoint_cursor, state.gql.endpoint.len());
        assert_eq!(state.ui.input_mode, InputMode::Editing);
    }

    #[test]
    fn test_gql_edit_query_sets_field_and_cursor() {
        let mut state = make_state();
        state.gql_edit_query();
        assert_eq!(state.gql.active_field, GqlField::Query);
        assert_eq!(state.gql.query_cursor, state.gql.query.len());
        assert_eq!(state.ui.input_mode, InputMode::Editing);
    }

    #[test]
    fn test_gql_edit_variables_sets_field_and_cursor() {
        let mut state = make_state();
        state.gql_edit_variables();
        assert_eq!(state.gql.active_field, GqlField::Variables);
        assert_eq!(state.gql.variables_cursor, state.gql.variables.len());
        assert_eq!(state.ui.input_mode, InputMode::Editing);
    }

    // ── Character input per field ─────────────────────────────────────────────

    #[test]
    fn test_gql_char_inserts_in_endpoint() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Endpoint;
        state.gql.endpoint = "https://".to_string();
        state.gql.endpoint_cursor = state.gql.endpoint.len();
        let original_len = state.gql.endpoint.len();
        state.gql_char('x');
        assert_eq!(state.gql.endpoint.len(), original_len + 1);
        assert!(state.gql.endpoint.ends_with('x'));
    }

    #[test]
    fn test_gql_char_inserts_in_query() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Query;
        state.gql.query = String::new();
        state.gql.query_cursor = 0;
        state.gql_char('{');
        assert_eq!(state.gql.query, "{");
    }

    #[test]
    fn test_gql_backspace_deletes_from_variables() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Variables;
        state.gql.variables = "{}".to_string();
        state.gql.variables_cursor = 2;
        state.gql_backspace();
        assert_eq!(state.gql.variables, "{");
        assert_eq!(state.gql.variables_cursor, 1);
    }

    // ── Scroll ────────────────────────────────────────────────────────────────

    #[test]
    fn test_gql_scroll_up_does_not_underflow() {
        let mut state = make_state();
        state.gql.response_scroll = 0;
        state.gql_scroll_up();
        assert_eq!(state.gql.response_scroll, 0);
    }

    #[test]
    fn test_gql_scroll_down_increments() {
        let mut state = make_state();
        state.gql_scroll_down();
        assert_eq!(state.gql.response_scroll, 1);
        state.gql_scroll_down();
        assert_eq!(state.gql.response_scroll, 2);
    }

    // ── Response handling ─────────────────────────────────────────────────────

    #[test]
    fn test_handle_gql_response_updates_state_for_correct_id() {
        let mut state = make_state();
        let id = state.next_id();
        state.gql.pending_request_id = Some(id);
        state.gql.is_loading = true;

        state.handle_gql_response(id, 200, r#"{"data":{"users":[]}}"#.to_string(), 55);

        assert_eq!(state.gql.response, r#"{"data":{"users":[]}}"#);
        assert_eq!(state.gql.time_ms, 55);
        assert!(!state.gql.is_loading);
        assert!(state.gql.pending_request_id.is_none());
    }

    #[test]
    fn test_handle_gql_response_wrong_id_ignored() {
        let mut state = make_state();
        let own_id = state.next_id();
        state.gql.pending_request_id = Some(own_id);
        state.gql.is_loading = true;
        state.gql.response = "original".to_string();

        state.handle_gql_response(own_id + 99, 200, "new content".to_string(), 10);

        // Must remain unchanged
        assert!(state.gql.is_loading);
        assert_eq!(state.gql.response, "original");
    }

    #[test]
    fn test_handle_gql_error_prefixes_error_message() {
        let mut state = make_state();
        let id = state.next_id();
        state.gql.pending_request_id = Some(id);
        state.gql.is_loading = true;

        state.handle_gql_error(id, "field 'x' not found".to_string(), 12);

        assert!(state.gql.response.starts_with("Error:"));
        assert!(state.gql.response.contains("field 'x' not found"));
        assert!(!state.gql.is_loading);
    }
}
