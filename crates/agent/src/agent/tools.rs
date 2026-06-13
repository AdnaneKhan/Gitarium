//! Tool schemas advertised to the model in every Messages request.

use serde_json::{json, Value};

pub(super) fn tools() -> Value {
    json!([
        {
            "name": "github_api",
            "description": super::prompts::get("tool_github_api"),
            "input_schema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
                        "description": "HTTP method"
                    },
                    "path": {
                        "type": "string",
                        "description": "API path starting with '/', optionally with a query string"
                    },
                    "body": {
                        "type": "object",
                        "description": "JSON request body, for POST/PUT/PATCH"
                    }
                },
                "required": ["method", "path"]
            }
        },
        {
            "name": "bash",
            "description": super::prompts::get("tool_bash"),
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line to run"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "code_search",
            "description": super::prompts::get("tool_code_search"),
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search terms, optionally with qualifiers, e.g. \"LIVE_GEN language:rust\" or \"path:src/ extension:ts useState\""
                    },
                    "repo": {
                        "type": "string",
                        "description": "Scope to one repository as owner/name"
                    },
                    "page": {
                        "type": "integer",
                        "description": "Result page, 1-based (30 files per page)"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "grep",
            "description": super::prompts::get("tool_grep"),
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search (default: all files)"
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Case-insensitive matching"
                    }
                },
                "required": ["pattern"]
            }
        },
        {
            "name": "find",
            "description": super::prompts::get("tool_find"),
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Filename glob, e.g. \"*.json\""
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search under (default /)"
                    }
                },
                "required": ["pattern"]
            }
        }
    ])
}
