//! The tools MCPad offers to AI agents.
//!
//! Every tool carries a schema of its arguments and a hint about what it
//! does. The hints help a model to pick the right tool and to understand that
//! the tools that change something need a confirmation of the user.

use serde_json::{json, Value};

use crate::protocol::{Tool, ToolAnnotations};

/// The name of the tool that lists the files of the vault.
pub const LIST_FILES: &str = "list_files";

/// The name of the tool that reads a part of a file.
pub const READ_FILE: &str = "read_file";

/// The name of the tool that reads a file completely.
pub const READ_FILE_FULL: &str = "read_file_full";

/// The name of the tool that searches the text of the vault.
pub const SEARCH: &str = "search";

/// The name of the tool that tells about the vault.
pub const VAULT_INFO: &str = "vault_info";

/// The name of the tool that writes a file.
pub const WRITE_FILE: &str = "write_file";

/// The name of the tool that removes a file.
pub const DELETE_FILE: &str = "delete_file";

/// Whether a tool changes the vault.
pub fn changes_the_vault(name: &str) -> bool {
    matches!(name, WRITE_FILE | DELETE_FILE)
}

/// The tools of the server.
pub fn catalogue() -> Vec<Tool> {
    vec![
        Tool {
            name: LIST_FILES.to_string(),
            title: Some("List the files of the vault".to_string()),
            description: "Lists every file of the vault with its size and its kind. \
                          Paths are relative to the root of the vault and use forward slashes."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            annotations: Some(read_only("List the files of the vault")),
        },
        Tool {
            name: READ_FILE.to_string(),
            title: Some("Read a file of the vault".to_string()),
            description: "Reads a file of the vault. By default the first lines of the file \
                          are returned; use `start_line` and `max_lines` to read another part \
                          of a long file, or `read_file_full` to get the whole text."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path of the file, relative to the root of the vault."
                    },
                    "start_line": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "The first line to read, counted from one. Defaults to 1."
                    },
                    "max_lines": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "The number of lines to read. Defaults to 2000."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            annotations: Some(read_only("Read a file of the vault")),
        },
        Tool {
            name: READ_FILE_FULL.to_string(),
            title: Some("Read a file completely".to_string()),
            description: "Reads the whole text of a file of the vault. Use it only when the \
                          file is known to be small, a long file is better read in parts."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path of the file, relative to the root of the vault."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            annotations: Some(read_only("Read a file of the vault")),
        },
        Tool {
            name: SEARCH.to_string(),
            title: Some("Search the text of the vault".to_string()),
            description: "Searches the text of every file of the vault and returns the matching \
                          lines with their file and their line number. The search is not a \
                          regular expression by default; set `regex` to true to use one."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The text to look for."
                    },
                    "regex": {
                        "type": "boolean",
                        "description": "Treat the query as a regular expression. Defaults to false."
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Compare the query with the text exactly. Defaults to false."
                    },
                    "path": {
                        "type": "string",
                        "description": "Look only inside this directory of the vault."
                    },
                    "max_results": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "The number of matches to return. Defaults to 200."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            annotations: Some(read_only("Search the text of the vault")),
        },
        Tool {
            name: VAULT_INFO.to_string(),
            title: Some("Tell about the vault".to_string()),
            description: "Returns the root of the vault, the number of its files and their total \
                          size. Useful as the first call of a session."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            annotations: Some(read_only("Tell about the vault")),
        },
        Tool {
            name: WRITE_FILE.to_string(),
            title: Some("Write a file of the vault".to_string()),
            description: "Writes a file of the vault and creates it when it does not exist yet. \
                          The user has to confirm the change, the tool fails when the change \
                          is refused. The file is written atomically and keeps the encoding of \
                          the vault."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path of the file, relative to the root of the vault."
                    },
                    "text": {
                        "type": "string",
                        "description": "The whole new content of the file."
                    },
                    "encoding": {
                        "type": "string",
                        "description": "The encoding of the file, for example `UTF-8` or \
                                       `Windows-1251`. The encoding of the file is kept when \
                                       it is not given."
                    },
                    "line_ending": {
                        "type": "string",
                        "description": "The line ending of the file, `LF` or `CRLF`. The line \
                                       ending of the file is kept when it is not given."
                    }
                },
                "required": ["path", "text"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Write a file of the vault".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
        },
        Tool {
            name: DELETE_FILE.to_string(),
            title: Some("Remove a file of the vault".to_string()),
            description: "Removes a file of the vault. The user has to confirm the change, the \
                          tool fails when the change is refused. Directories are never removed."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path of the file, relative to the root of the vault."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Remove a file of the vault".to_string()),
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: true,
                open_world_hint: false,
            }),
        },
    ]
}

/// The hints of a tool that only reads.
fn read_only(title: &str) -> ToolAnnotations {
    ToolAnnotations {
        title: Some(title.to_string()),
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: false,
    }
}

/// The schema of a tool, for the tests and for the documentation.
pub fn schema_of(name: &str) -> Option<Value> {
    catalogue()
        .into_iter()
        .find(|tool| tool.name == name)
        .map(|tool| tool.input_schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_holds_every_tool_of_the_server() {
        let names: Vec<String> = catalogue().into_iter().map(|tool| tool.name).collect();
        assert_eq!(
            names,
            vec![
                LIST_FILES,
                READ_FILE,
                READ_FILE_FULL,
                SEARCH,
                VAULT_INFO,
                WRITE_FILE,
                DELETE_FILE
            ]
        );
    }

    #[test]
    fn every_tool_has_a_schema_with_an_object_of_properties() {
        for tool in catalogue() {
            assert_eq!(tool.input_schema["type"], "object");
            assert!(tool.input_schema["properties"].is_object());
        }
    }

    #[test]
    fn only_two_tools_change_the_vault() {
        assert!(changes_the_vault(WRITE_FILE));
        assert!(changes_the_vault(DELETE_FILE));
        assert!(!changes_the_vault(READ_FILE));
        assert!(!changes_the_vault(SEARCH));
        assert!(!changes_the_vault(LIST_FILES));
    }

    #[test]
    fn the_tools_that_change_the_vault_are_not_marked_as_read_only() {
        for tool in catalogue() {
            let annotations = tool.annotations.expect("every tool has hints");
            if changes_the_vault(&tool.name) {
                assert!(
                    !annotations.read_only_hint,
                    "{} is not read only",
                    tool.name
                );
            } else {
                assert!(annotations.read_only_hint, "{} is read only", tool.name);
            }
        }
    }

    #[test]
    fn the_tools_that_write_need_their_arguments() {
        let schema = schema_of(WRITE_FILE).unwrap();
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|value| value == "path"));
        assert!(required.iter().any(|value| value == "text"));
    }
}
