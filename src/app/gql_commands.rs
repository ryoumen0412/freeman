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
            headers: self.request.headers.clone(),
            auth: self.request.auth.clone(),
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
        self.input_mode = InputMode::Editing;
    }

    /// Start editing GraphQL query
    pub fn gql_edit_query(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = GqlField::Query;
        self.gql.query_cursor = self.gql.query.len();
        self.input_mode = InputMode::Editing;
    }

    /// Start editing GraphQL variables
    pub fn gql_edit_variables(&mut self) {
        use crate::messages::ui_events::GqlField;
        self.gql.active_field = GqlField::Variables;
        self.gql.variables_cursor = self.gql.variables.len();
        self.input_mode = InputMode::Editing;
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
