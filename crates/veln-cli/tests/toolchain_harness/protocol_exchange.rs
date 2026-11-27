use super::*;
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

// The deadline covers the complete conversation, including writes and EOF.
// Reader threads drain both pipes; Drop kills/reaps descendants on every failure.
pub(super) fn exchange(command: Command, stdin: &str, lsp: bool) -> Output {
    exchange_with_timeout(command, stdin, lsp, TIMEOUT)
}

fn exchange_with_timeout(
    mut command: Command,
    stdin: &str,
    lsp: bool,
    timeout: Duration,
) -> Output {
    let requests = if lsp {
        decode_lsp_stdout(stdin).expect("interactive LSP requests should decode")
    } else {
        stdin
            .lines()
            .map(|line| parse_json(line).expect("MCP request should be JSON"))
            .collect()
    };
    let session = Session::new(&mut command, lsp, timeout);
    let mut responses = Vec::new();
    let mut stdout = Vec::new();
    for mut request in requests {
        expand_response_directives(&mut request, &responses).unwrap_or_else(|error| {
            panic!(
                "cannot resolve request {}: {error}",
                request.to_compact_string()
            )
        });
        let body =
            super::project_fixture::substitute_mcp_cursor(&request.to_compact_string(), &responses);
        let framed = if lsp {
            lsp_frame(&body)
        } else {
            format!("{body}\n")
        };
        session.send(framed);
        if let Some(id) = request.object_field("id") {
            loop {
                let (bytes, response) = session.receive();
                let matched = is_lsp_response(&response) && response.object_field("id") == Some(id);
                stdout.extend_from_slice(&bytes);
                responses.push(response);
                if matched {
                    break;
                }
            }
        }
    }
    session.finish(stdout)
}

struct Session {
    child: std::process::Child,
    input: Option<mpsc::Sender<String>>,
    output: mpsc::Receiver<Result<(Vec<u8>, JsonValue), String>>,
    writer: Option<std::thread::JoinHandle<()>>,
    reader: Option<std::thread::JoinHandle<()>>,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
    deadline: Instant,
    reaped: bool,
}

impl Session {
    fn new(command: &mut Command, lsp: bool, timeout: Duration) -> Self {
        let mut child = command.spawn().expect("protocol process should spawn");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut stderr = child.stderr.take().unwrap();
        let (input, writes) = mpsc::channel::<String>();
        let writer = std::thread::spawn(move || {
            for request in writes {
                if stdin
                    .write_all(request.as_bytes())
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let (sender, output) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            loop {
                let result = read_message(&mut stdout, lsp);
                match result {
                    Ok(None) => break,
                    Ok(Some(message)) => {
                        if sender.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr
                .read_to_end(&mut bytes)
                .expect("protocol stderr should be readable");
            bytes
        });
        Self {
            child,
            input: Some(input),
            output,
            writer: Some(writer),
            reader: Some(reader),
            stderr: Some(stderr),
            deadline: Instant::now() + timeout,
            reaped: false,
        }
    }

    fn send(&self, request: String) {
        self.input
            .as_ref()
            .unwrap()
            .send(request)
            .expect("protocol writer should be available");
    }

    fn receive(&self) -> (Vec<u8>, JsonValue) {
        self.output.recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
            .expect("protocol response missing before deadline; inspect the request id and server output")
            .expect("protocol stdout must contain complete JSON-RPC messages")
    }

    fn finish(mut self, mut stdout: Vec<u8>) -> Output {
        drop(self.input.take());
        let status = loop {
            if let Some(status) = self
                .child
                .try_wait()
                .expect("protocol process status should be available")
            {
                break status;
            }
            assert!(
                Instant::now() < self.deadline,
                "protocol process did not exit before deadline; inspect shutdown and EOF handling"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        super::project_fixture::terminate_fixture_process_tree(&mut self.child);
        self.reaped = true;
        self.join_io();
        for result in self.output.try_iter() {
            let (bytes, _) = result.expect("trailing protocol output should decode");
            stdout.extend_from_slice(&bytes);
        }
        let stderr = self
            .stderr
            .take()
            .unwrap()
            .join()
            .expect("stderr reader should finish");
        Output {
            status,
            stdout,
            stderr,
        }
    }

    fn join_io(&mut self) {
        if let Some(writer) = self.writer.take() {
            writer.join().expect("protocol writer should finish");
        }
        if let Some(reader) = self.reader.take() {
            reader.join().expect("protocol reader should finish");
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        drop(self.input.take());
        if !self.reaped {
            super::project_fixture::terminate_fixture_process_tree(&mut self.child);
            let _ = self.child.wait();
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.join();
        }
    }
}

fn read_message(
    reader: &mut impl BufRead,
    lsp: bool,
) -> Result<Option<(Vec<u8>, JsonValue)>, String> {
    let mut bytes = Vec::new();
    if lsp {
        loop {
            let mut line = String::new();
            let size = reader
                .read_line(&mut line)
                .map_err(|error| error.to_string())?;
            if size == 0 {
                return if bytes.is_empty() {
                    Ok(None)
                } else {
                    Err("partial LSP header".into())
                };
            }
            bytes.extend_from_slice(line.as_bytes());
            if line == "\r\n" {
                break;
            }
        }
        let (start, length) = decode_lsp_header(&bytes, 0, "incomplete LSP header")?;
        bytes.resize(start.checked_add(length).ok_or("LSP length overflow")?, 0);
        reader
            .read_exact(&mut bytes[start..])
            .map_err(|error| error.to_string())?;
        let (value, _) = decode_lsp_frame(&bytes, 0, "incomplete LSP frame")?;
        Ok(Some((bytes, value)))
    } else {
        if reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Ok(None);
        }
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        let value = parse_json(text).map_err(|error| error.to_string())?;
        Ok(Some((bytes, value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_reader_preserves_frames_and_rejects_truncation() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":"é😀"}"#;
        let frame = lsp_frame(body);
        let (bytes, message) = read_message(&mut frame.as_bytes(), true).unwrap().unwrap();
        assert_eq!(bytes, frame.as_bytes());
        assert_eq!(message, parse_json(body).unwrap());
        for input in [
            "Content-Length: 2\r\n",
            "Content-Length: 100\r\n\r\n{}",
            "Content-Length: 1\r\n\r\n{",
        ] {
            assert!(read_message(&mut input.as_bytes(), true).is_err());
        }
        assert!(read_message(&mut b"not JSON\n".as_slice(), false).is_err());
        assert!(read_message(&mut b"".as_slice(), true).unwrap().is_none());
    }

    #[cfg(unix)]
    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        command
    }

    #[cfg(unix)]
    #[test]
    fn interactive_exchange_reuses_uri_and_keeps_notifications() {
        let command = shell(
            r#"read -r first
printf '%s\n' '{"jsonrpc":"2.0","method":"notice"}' '{"jsonrpc":"2.0","id":1,"result":{"uri":"veln-pkg:///snapshot/new/é.veln"}}'
read -r second
case "$second" in
  *'veln-pkg:///snapshot/new/é.veln'*) printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":"ok"}' ;;
  *) exit 4 ;;
esac
"#,
        );
        let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"first\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"second\",\"params\":{\"uri\":{\"$response\":{\"id\":1,\"path\":\"/result/uri\"}}}}\n";
        let output = exchange_with_timeout(command, input, false, Duration::from_secs(2));
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 3);
        assert!(stdout.contains("notice"));
        assert!(output.stderr.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn interactive_exchange_deadline_reaps_descendant_pipe_holders() {
        let started = Instant::now();
        let failure = std::panic::catch_unwind(|| {
            exchange_with_timeout(
                shell("sleep 10 & wait"),
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"first\"}\n",
                false,
                Duration::from_millis(50),
            )
        });
        assert!(failure.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
