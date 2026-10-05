//! Every user-facing operation, so the command palette and help can list them all and
//! nothing is reachable only through a key the terminal might not deliver.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Send,
    CancelRequest,
    Save,
    SaveAs,
    NewRequest,
    OpenRequest,
    RenameRequest,
    DuplicateRequest,
    DeleteRequest,
    FindRequest,
    FocusUrl,
    FocusBody,
    FocusResponse,
    ChangeMethod,
    ToggleSecure,
    FormatBody,
    SwitchEnvironment,
    ShowRequests,
    ShowEnvironments,
    NewEnvironment,
    EditAuthorization,
    ToggleSidebar,
    ToggleZoom,
    SearchResponse,
    CopyResponse,
    SaveResponse,
    SaveResponseTimestamped,
    OpenLastResponse,
    ShowCliCommand,
    ShowCurlCommand,
    ImportOpenApi,
    Reload,
    CreateProject,
    Help,
    Quit,
}

impl Action {
    pub const ALL: [Action; 35] = [
        Action::Send,
        Action::Save,
        Action::NewRequest,
        Action::SwitchEnvironment,
        Action::FindRequest,
        Action::OpenRequest,
        Action::SaveAs,
        Action::RenameRequest,
        Action::DuplicateRequest,
        Action::DeleteRequest,
        Action::FocusUrl,
        Action::FocusBody,
        Action::FocusResponse,
        Action::ChangeMethod,
        Action::ToggleSecure,
        Action::FormatBody,
        Action::CancelRequest,
        Action::SearchResponse,
        Action::CopyResponse,
        Action::SaveResponse,
        Action::SaveResponseTimestamped,
        Action::OpenLastResponse,
        Action::ToggleZoom,
        Action::ShowCliCommand,
        Action::ShowCurlCommand,
        Action::ShowRequests,
        Action::ShowEnvironments,
        Action::NewEnvironment,
        Action::EditAuthorization,
        Action::ToggleSidebar,
        Action::ImportOpenApi,
        Action::Reload,
        Action::CreateProject,
        Action::Help,
        Action::Quit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::Send => "Send request",
            Action::CancelRequest => "Cancel running request",
            Action::Save => "Save request",
            Action::SaveAs => "Save request as…",
            Action::NewRequest => "New request",
            Action::OpenRequest => "Open highlighted request",
            Action::RenameRequest => "Rename saved request",
            Action::DuplicateRequest => "Duplicate saved request",
            Action::DeleteRequest => "Delete saved request",
            Action::FindRequest => "Find saved request",
            Action::FocusUrl => "Edit URL",
            Action::FocusBody => "Edit body",
            Action::FocusResponse => "Go to response",
            Action::ChangeMethod => "Change method",
            Action::ToggleSecure => "Toggle sending project authorization",
            Action::FormatBody => "Format JSON body",
            Action::SwitchEnvironment => "Switch environment",
            Action::ShowRequests => "Show requests",
            Action::ShowEnvironments => "Manage environments",
            Action::NewEnvironment => "New environment",
            Action::EditAuthorization => "Edit authorization of active environment",
            Action::ToggleSidebar => "Toggle request list",
            Action::ToggleZoom => "Toggle full-screen response",
            Action::SearchResponse => "Search response",
            Action::CopyResponse => "Copy response to clipboard",
            Action::SaveResponse => "Save response to file",
            Action::SaveResponseTimestamped => "Save response to timestamped file",
            Action::OpenLastResponse => "Open last saved response",
            Action::ShowCliCommand => "Show as gemon CLI command",
            Action::ShowCurlCommand => "Show as cURL command",
            Action::ImportOpenApi => "Import OpenAPI (openapi.yaml)",
            Action::Reload => "Reload project from disk",
            Action::CreateProject => "Create project in this folder",
            Action::Help => "Keyboard shortcuts",
            Action::Quit => "Quit",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Action::Send => "Ctrl+R",
            Action::CancelRequest => "Esc",
            Action::Save => "Ctrl+S",
            Action::NewRequest => "Ctrl+N",
            Action::RenameRequest => "r",
            Action::DuplicateRequest => "c",
            Action::DeleteRequest => "d",
            Action::OpenRequest => "Enter",
            Action::FindRequest => "Ctrl+F",
            Action::FocusUrl => "Ctrl+L",
            Action::ChangeMethod => "Ctrl+T",
            Action::SwitchEnvironment => "Ctrl+G",
            Action::ShowRequests => "F1",
            Action::ShowEnvironments => "F2",
            Action::ToggleZoom => "z",
            Action::SearchResponse => "/",
            Action::CopyResponse => "y",
            Action::SaveResponse => "s",
            Action::ImportOpenApi => "Ctrl+O",
            Action::Help => "F3  ?",
            Action::Quit => "Ctrl+Q",
            _ => "",
        }
    }
}

/// Palette match score; every query word must appear in the label. Higher is better and
/// `None` means no match. Words matching at a word start rank first.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let text = text.to_lowercase();
    let mut score = 0;
    for word in query.to_lowercase().split_whitespace() {
        let position = text.find(word)?;
        let at_word_start = !matches!(
            text[..position].chars().last(),
            Some(previous) if previous.is_alphanumeric()
        );
        score += if at_word_start { 100 } else { 10 };
        score -= position as i32;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::{fuzzy_score, Action};

    #[test]
    fn every_action_is_listed_once() {
        for action in Action::ALL {
            assert_eq!(
                Action::ALL.iter().filter(|other| **other == action).count(),
                1,
                "{action:?}"
            );
        }
    }

    #[test]
    fn fuzzy_score_matches_words_and_ranks_word_starts_first() {
        assert!(fuzzy_score("", "Switch environment").is_some());
        assert!(fuzzy_score("sw env", "Switch environment").is_some());
        assert!(fuzzy_score("env", "Rename saved request").is_none());
        assert!(fuzzy_score("xyz", "Switch environment").is_none());
        assert!(fuzzy_score("send", "Send request") > fuzzy_score("end", "Send request"));
        assert!(fuzzy_score("save", "Save request") > fuzzy_score("save", "Open last saved response"));
    }
}
