//! LSP-facing semantic token helpers for Veln editors.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use veln_editor::{encode_lsp_semantic_tokens, semantic_token_legend};
use veln_source::SourceFile;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTokensLegend {
    pub token_types: Vec<&'static str>,
    pub token_modifiers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTokensFull {
    pub data: Vec<u32>,
}

pub fn legend() -> SemanticTokensLegend {
    let (token_types, token_modifiers) = semantic_token_legend();
    SemanticTokensLegend {
        token_types,
        token_modifiers,
    }
}

pub fn semantic_tokens_full(source: &SourceFile) -> SemanticTokensFull {
    let tokens = veln_editor::collect_semantic_tokens(source);
    let data = encode_lsp_semantic_tokens(&tokens)
        .into_iter()
        .flat_map(|token| {
            [
                token.delta_line,
                token.delta_start,
                token.length,
                token.token_type,
                token.token_modifiers,
            ]
        })
        .collect();
    SemanticTokensFull { data }
}

pub fn run_stdio() -> io::Result<()> {
    Server::default().run(io::stdin().lock(), io::stdout().lock())
}

#[derive(Default)]
struct Server {
    documents: BTreeMap<String, String>,
    should_exit: bool,
}

impl Server {
    fn run(&mut self, input: impl Read, mut output: impl Write) -> io::Result<()> {
        let mut input = BufReader::new(input);
        while !self.should_exit {
            let Some(message) = read_message(&mut input)? else {
                break;
            };
            for response in self.handle_message(&message) {
                write_message(&mut output, &response)?;
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, message: &str) -> Vec<String> {
        let method = extract_string_field(message, "method");
        let id = extract_id(message);
        let Some(method) = method.as_deref() else {
            return Vec::new();
        };

        match method {
            "initialize" => id
                .map(|id| response(&id, &initialize_result()))
                .into_iter()
                .collect(),
            "initialized" => Vec::new(),
            "shutdown" => id.map(|id| response(&id, "null")).into_iter().collect(),
            "exit" => {
                self.should_exit = true;
                Vec::new()
            }
            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    extract_string_field(message, "uri"),
                    extract_string_field(message, "text"),
                ) {
                    self.documents.insert(uri, text);
                }
                Vec::new()
            }
            "textDocument/didChange" => {
                if let (Some(uri), Some(text)) = (
                    extract_string_field(message, "uri"),
                    extract_string_field(message, "text"),
                ) {
                    self.documents.insert(uri, text);
                }
                Vec::new()
            }
            "textDocument/semanticTokens/full" => id
                .map(|id| {
                    let uri = extract_string_field(message, "uri").unwrap_or_default();
                    response(&id, &semantic_tokens_result(&uri, self.document_text(&uri)))
                })
                .into_iter()
                .collect(),
            _ => id
                .map(|id| error_response(&id, -32601, "method not found"))
                .into_iter()
                .collect(),
        }
    }

    fn document_text(&self, uri: &str) -> String {
        if let Some(text) = self.documents.get(uri) {
            return text.clone();
        }
        uri_to_path(uri)
            .and_then(|path| fs::read_to_string(path).ok())
            .unwrap_or_default()
    }
}

fn initialize_result() -> String {
    let legend = legend();
    format!(
        "{{\"capabilities\":{{\"textDocumentSync\":1,\"semanticTokensProvider\":{{\"legend\":{{\"tokenTypes\":[{}],\"tokenModifiers\":[{}]}},\"full\":true,\"range\":false}}}}}}",
        json_string_list(&legend.token_types),
        json_string_list(&legend.token_modifiers),
    )
}

fn semantic_tokens_result(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text);
    let data = semantic_tokens_full(&source)
        .data
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"data\":[{data}]}}")
}

fn read_message(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(content_length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; content_length];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn write_message(output: &mut impl Write, body: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

fn response(id: &str, result: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}")
}

fn error_response(id: &str, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":\"{}\"}}}}",
        escape_json(message)
    )
}

fn extract_id(message: &str) -> Option<String> {
    let key = "\"id\"";
    let index = message.find(key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    if after_colon.starts_with('"') {
        let value = parse_json_string(after_colon)?;
        Some(format!("\"{}\"", escape_json(&value)))
    } else {
        let end = after_colon
            .find(|ch: char| !ch.is_ascii_digit() && ch != '-')
            .unwrap_or(after_colon.len());
        Some(after_colon[..end].to_string())
    }
}

fn extract_string_field(message: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    parse_json_string(after_colon)
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                '/' => value.push('/'),
                'b' => value.push('\u{0008}'),
                'f' => value.push('\u{000c}'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    let code = chars.by_ref().take(4).collect::<String>();
                    let Ok(value_code) = u32::from_str_radix(&code, 16) else {
                        return None;
                    };
                    value.push(char::from_u32(value_code)?);
                }
                _ => return None,
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn json_string_list(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            ch => vec![ch],
        })
        .collect()
}

fn display_path(uri: &str) -> String {
    uri_to_path(uri)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| uri.to_string())
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let code = chars.by_ref().take(2).collect::<String>();
            if let Ok(byte) = u8::from_str_radix(&code, 16) {
                output.push(byte as char);
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_exposes_standard_types_and_custom_modifiers() {
        let legend = legend();

        assert!(legend.token_types.contains(&"function"));
        assert!(legend.token_types.contains(&"parameter"));
        assert!(legend.token_types.contains(&"namespace"));
        assert!(legend.token_modifiers.contains(&"declaration"));
        assert!(legend.token_modifiers.contains(&"defaultLibrary"));
        assert!(legend.token_modifiers.contains(&"test"));
        assert!(legend.token_modifiers.contains(&"result"));
        assert!(legend.token_modifiers.contains(&"hole"));
    }

    #[test]
    fn full_tokens_are_flat_lsp_integer_data() {
        let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");

        let response = semantic_tokens_full(&source);

        assert_eq!(response.data.len() % 5, 0);
        assert!(response.data.len() >= 10);
    }

    #[test]
    fn server_initializes_with_semantic_token_capability() {
        let mut server = Server::default();

        let responses =
            server.handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""semanticTokensProvider""#));
        assert!(responses[0].contains(r#""tokenTypes":["namespace","type""#));
    }

    #[test]
    fn server_returns_full_semantic_tokens_for_open_document() {
        let mut server = Server::default();
        server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn main() -> Int\n  main()\nend\n"}}}"#,
        );

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""id":2"#));
        assert!(responses[0].contains(r#""data":["#));
    }

    #[test]
    fn server_reads_and_writes_content_length_frames() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let input = format!("Content-Length: {}\r\n\r\n{request}", request.len());
        let mut output = Vec::new();
        let mut server = Server::default();

        server.run(input.as_bytes(), &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains(r#""id":1"#));
    }
}
