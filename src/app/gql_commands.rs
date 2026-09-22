//! GraphQL orchestration tests.
//!
//! Pure GQL domain logic lives in `gql_state.rs` (impl GraphQLState).
//! GQL orchestration methods that touch multiple sub-states are in
//! `http_commands.rs` (gql_execute_query, gql_edit_endpoint, etc.).
//!
//! This file retains integration tests that verify the full AppState flow.

#[cfg(test)]
mod tests {
    use crate::app::AppState;
    use crate::messages::ui_events::{GqlField, InputMode};

    fn make_state() -> AppState {
        AppState::new()
    }

    // ── Execute query (via AppState orchestration) ────────────────────────────

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

    // ── Field cycling (direct on sub-state) ──────────────────────────────────

    #[test]
    fn test_gql_next_field_cycles_endpoint_query_variables() {
        let mut state = make_state();
        assert_eq!(state.gql.active_field, GqlField::Query);
        state.gql.next_field();
        assert_eq!(state.gql.active_field, GqlField::Variables);
        state.gql.next_field();
        assert_eq!(state.gql.active_field, GqlField::Endpoint);
        state.gql.next_field();
        assert_eq!(state.gql.active_field, GqlField::Query);
    }

    // ── Edit mode entry (via AppState orchestration) ─────────────────────────

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

    // ── Character input per field (direct on sub-state) ──────────────────────

    #[test]
    fn test_gql_char_inserts_in_endpoint() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Endpoint;
        state.gql.endpoint = "https://".to_string();
        state.gql.endpoint_cursor = state.gql.endpoint.len();
        let original_len = state.gql.endpoint.len();
        state.gql.char_input('x');
        assert_eq!(state.gql.endpoint.len(), original_len + 1);
        assert!(state.gql.endpoint.ends_with('x'));
    }

    #[test]
    fn test_gql_char_inserts_in_query() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Query;
        state.gql.query = String::new();
        state.gql.query_cursor = 0;
        state.gql.char_input('{');
        assert_eq!(state.gql.query, "{");
    }

    #[test]
    fn test_gql_backspace_deletes_from_variables() {
        let mut state = make_state();
        state.gql.active_field = GqlField::Variables;
        state.gql.variables = "{}".to_string();
        state.gql.variables_cursor = 2;
        state.gql.backspace();
        assert_eq!(state.gql.variables, "{");
        assert_eq!(state.gql.variables_cursor, 1);
    }

    // ── Scroll (direct on sub-state) ─────────────────────────────────────────

    #[test]
    fn test_gql_scroll_up_does_not_underflow() {
        let mut state = make_state();
        state.gql.response_scroll = 0;
        state.gql.scroll_up();
        assert_eq!(state.gql.response_scroll, 0);
    }

    #[test]
    fn test_gql_scroll_down_increments() {
        let mut state = make_state();
        state.gql.scroll_down();
        assert_eq!(state.gql.response_scroll, 1);
        state.gql.scroll_down();
        assert_eq!(state.gql.response_scroll, 2);
    }

    // ── Response handling (direct on sub-state) ──────────────────────────────

    #[test]
    fn test_handle_gql_response_updates_state_for_correct_id() {
        let mut state = make_state();
        let id = state.next_id();
        state.gql.pending_request_id = Some(id);
        state.gql.is_loading = true;

        state
            .gql
            .apply_response(id, r#"{"data":{"users":[]}}"#.to_string(), 55);

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

        state
            .gql
            .apply_response(own_id + 99, "new content".to_string(), 10);

        assert!(state.gql.is_loading);
        assert_eq!(state.gql.response, "original");
    }

    #[test]
    fn test_handle_gql_error_prefixes_error_message() {
        let mut state = make_state();
        let id = state.next_id();
        state.gql.pending_request_id = Some(id);
        state.gql.is_loading = true;

        state
            .gql
            .apply_error(id, "field 'x' not found".to_string(), 12);

        assert!(state.gql.response.starts_with("Error:"));
        assert!(state.gql.response.contains("field 'x' not found"));
        assert!(!state.gql.is_loading);
    }
}
