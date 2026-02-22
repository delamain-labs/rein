//! Language Server Protocol implementation for Rein.
//!
//! Provides real-time diagnostics, hover info, and completion
//! for `.rein` files in any LSP-compatible editor.

use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, MarkupContent, MarkupKind, MessageType,
    Position, Range, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::parser::parse;
use crate::validator;

#[cfg(test)]
mod tests;

/// The Rein language server backend.
pub struct ReinLanguageServer {
    client: Client,
    /// Cache of open document contents.
    documents: Mutex<std::collections::HashMap<Url, String>>,
}

impl ReinLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Validate a document and publish diagnostics.
    async fn validate_document(&self, uri: Url, text: &str) {
        let diagnostics = compute_diagnostics(text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ReinLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), " ".to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Rein language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.documents
            .lock()
            .expect("documents mutex poisoned")
            .insert(uri.clone(), text.clone());
        self.validate_document(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text.clone();
            self.documents
                .lock()
                .expect("documents mutex poisoned")
                .insert(uri.clone(), text.clone());
            self.validate_document(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .expect("documents mutex poisoned")
            .remove(&params.text_document.uri);
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let items = KEYWORD_COMPLETIONS
            .iter()
            .map(|(label, detail)| CompletionItem {
                label: (*label).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some((*detail).to_string()),
                ..CompletionItem::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.lock().expect("documents mutex poisoned");
        let Some(text) = docs.get(uri) else {
            return Ok(None);
        };

        let word = word_at_position(text, pos);
        let hover_text = keyword_docs(&word);

        Ok(hover_text.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text,
            }),
            range: None,
        }))
    }
}

/// Compute LSP diagnostics from Rein source text.
pub fn compute_diagnostics(text: &str) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let mut diagnostics = Vec::new();

    // Parse errors
    if let Err(e) = parse(text) {
        let (line, col) = offset_to_line_col(text, e.span.start);
        diagnostics.push(tower_lsp::lsp_types::Diagnostic {
            range: Range {
                start: Position::new(line, col),
                end: Position::new(line, col + 1),
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("rein".to_string()),
            message: e.message.clone(),
            ..tower_lsp::lsp_types::Diagnostic::default()
        });
        return diagnostics;
    }

    // Validation warnings/errors
    if let Ok(file) = parse(text) {
        for diag in validator::validate(&file) {
            let (line, col) = offset_to_line_col(text, diag.span.start);
            let severity = if diag.is_error() {
                DiagnosticSeverity::ERROR
            } else {
                DiagnosticSeverity::WARNING
            };
            diagnostics.push(tower_lsp::lsp_types::Diagnostic {
                range: Range {
                    start: Position::new(line, col),
                    end: Position::new(line, col + 1),
                },
                severity: Some(severity),
                source: Some("rein".to_string()),
                message: diag.message.clone(),
                ..tower_lsp::lsp_types::Diagnostic::default()
            });
        }
    }

    diagnostics
}

fn offset_to_line_col(text: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn word_at_position(text: &str, pos: Position) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return String::new();
    }
    let line = lines[line_idx];
    let col = pos.character as usize;
    if col >= line.len() {
        return String::new();
    }

    let bytes = line.as_bytes();
    let mut start = col;
    let mut end = col;

    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    line[start..end].to_string()
}

const KEYWORD_COMPLETIONS: &[(&str, &str)] = &[
    (
        "agent",
        "Define an AI agent with capabilities and constraints",
    ),
    ("workflow", "Define a multi-step workflow"),
    ("step", "A unit of work within a workflow"),
    ("provider", "Configure an AI model provider"),
    ("defaults", "Set project-wide defaults"),
    ("archetype", "Reusable agent template"),
    ("policy", "Conditional governance rules"),
    ("type", "Custom type definition"),
    ("tool", "External tool integration"),
    ("guardrails", "Safety constraints"),
    ("circuit_breaker", "Failure detection and recovery"),
    ("observe", "Observability configuration"),
    ("fleet", "Multi-agent group definition"),
    ("channel", "Communication channel"),
    ("eval", "Quality gate with assertions"),
    ("consensus", "Multi-agent verification"),
    ("approval", "Human-in-the-loop gate"),
    ("escalate", "Agent-to-human handoff"),
    ("secrets", "Vault-based secret management"),
    ("memory", "Agent memory system"),
    ("schedule", "Time-based triggers"),
    ("scenario", "Declarative test definition"),
    ("import", "Import from other .rein files"),
    ("model", "AI model to use"),
    ("can", "Allowed capabilities"),
    ("cannot", "Denied capabilities"),
    ("budget", "Spending limit"),
    ("trigger", "Event that starts a workflow"),
    ("goal", "Task description for a step"),
    ("when", "Guard condition"),
    ("from", "Inherit from archetype"),
    ("env", "Environment variable reference"),
];

fn keyword_docs(word: &str) -> Option<String> {
    let docs = match word {
        "agent" => {
            "## agent\n\nDefine an AI agent with model, capabilities, and constraints.\n\n```rein\nagent name {\n    model: gpt-4\n    can: action1, action2\n    cannot: action3\n    budget: $100 per day\n}\n```"
        }
        "workflow" => {
            "## workflow\n\nDefine a multi-step workflow with triggers and steps.\n\n```rein\nworkflow name {\n    trigger: event\n    step classify { agent: bot, goal: \"...\" }\n}\n```"
        }
        "step" => {
            "## step\n\nA unit of work within a workflow.\n\n```rein\nstep name {\n    agent: agent_name\n    goal: \"Task description\"\n    when: condition\n}\n```"
        }
        "provider" => {
            "## provider\n\nConfigure an AI model provider.\n\n```rein\nprovider openai {\n    model: gpt-4\n    key: env(\"OPENAI_API_KEY\")\n}\n```"
        }
        "budget" => {
            "## budget\n\nSpending constraint with currency and time period.\n\n```rein\nbudget: $100 per day\nbudget: €500 per month\n```"
        }
        "can" | "cannot" => {
            "## can / cannot\n\nCapability lists. Use `up_to` for constrained capabilities.\n\n```rein\ncan: read_files, search_web\ncan: issue_refunds up_to $500\ncannot: delete_data\n```"
        }
        "when" => {
            "## when\n\nGuard condition on steps. Supports comparison operators and boolean logic.\n\n```rein\nwhen: confidence < 70%\nwhen: status == \"critical\" and priority > 3\n```"
        }
        "env" => {
            "## env()\n\nReference an environment variable with optional default.\n\n```rein\nkey: env(\"API_KEY\")\nkey: env(\"API_KEY\", \"default_value\")\n```"
        }
        _ => return None,
    };
    Some(docs.to_string())
}

/// Start the LSP server on stdin/stdout.
pub async fn run_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ReinLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
