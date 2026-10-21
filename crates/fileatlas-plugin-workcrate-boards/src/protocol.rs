use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub(crate) struct Client {
    stream: BufReader<TcpStream>,
    next_id: u64,
    pub bound_path: Option<PathBuf>,
    poisoned: bool,
    pub diagnostics: Arc<Mutex<BTreeMap<PathBuf, Value>>>,
}
pub(crate) trait NativeProtocol {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, BoardsError>;
    fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError>;
}
impl NativeProtocol for Client {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        Client::request(self, method, params)
    }
    fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError> {
        Client::execute(self, command, params)
    }
}
impl Client {
    pub fn connect(port: u16) -> Result<Self, BoardsError> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(65)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        Ok(Self {
            stream: BufReader::new(stream),
            next_id: 1,
            bound_path: None,
            poisoned: false,
            diagnostics: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        if self.poisoned {
            return Err(BoardsError::Protocol(
                "connection invalidated after uncertain outcome".into(),
            ));
        }
        let result = self.request_inner(method, params);
        if matches!(
            &result,
            Err(BoardsError::Io(_) | BoardsError::Json(_) | BoardsError::Protocol(_))
        ) {
            self.poisoned = true;
            let _ = self.stream.get_mut().shutdown(std::net::Shutdown::Both);
        }
        result
    }
    fn request_inner(&mut self, method: &str, params: Value) -> Result<Value, BoardsError> {
        if let Some(path) = self.bound_path.clone() {
            let mutation = method == "app.save"
                || method == "app.export"
                || method == "ui.render"
                || method == "ui.focus"
                || method == "engine.execute"
                    && !params["command"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("document.") || s == "attributes.info");
            if mutation {
                let current = self.request("document.inspect", json!({"depth":0}))?;
                let raw = current["path"].as_str().unwrap_or("");
                let normalize = |p: &str| {
                    p.trim_start_matches(r"\\?\")
                        .replace('\\', "/")
                        .to_lowercase()
                };
                if normalize(raw) != normalize(&path.to_string_lossy()) {
                    return Err(BoardsError::Invalid(
                        "active document changed during transaction".into(),
                    ));
                }
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut bytes = serde_json::to_vec(&json!({"id":id,"method":method,"params":params}))?;
        if bytes.len() > 4 * 1024 * 1024 - 1 {
            return Err(BoardsError::Protocol("request exceeds limit".into()));
        }
        bytes.push(b'\n');
        self.stream.get_mut().write_all(&bytes)?;
        let mut reply = Vec::new();
        loop {
            let available = self.stream.fill_buf()?;
            if available.is_empty() {
                return Err(BoardsError::Protocol(
                    "connection closed; outcome uncertain".into(),
                ));
            }
            let n = available
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(available.len());
            if reply.len() + n > 32 * 1024 * 1024 {
                return Err(BoardsError::Protocol("reply exceeds limit".into()));
            }
            reply.extend_from_slice(&available[..n]);
            let done = available[n - 1] == b'\n';
            self.stream.consume(n);
            if done {
                break;
            }
        }
        let value: Value = serde_json::from_slice(&reply)?;
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            return Err(BoardsError::Protocol(
                "response ID mismatch; outcome uncertain".into(),
            ));
        }
        if value.get("ok").and_then(Value::as_bool).is_none() {
            return Err(BoardsError::Protocol(
                "response success discriminator missing".into(),
            ));
        }
        if value.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(BoardsError::Remote(
                value.get("error").unwrap_or(&Value::Null).to_string(),
            ));
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }
    pub fn execute(&mut self, command: &str, params: Value) -> Result<Value, BoardsError> {
        self.request("engine.execute", json!({"command":command,"params":params}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn failed_exchange(response: &'static [u8], inspect: bool) -> TestResult {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let worker = std::thread::spawn(move || -> std::io::Result<()> {
            let (s, _) = listener.accept()?;
            s.set_read_timeout(Some(Duration::from_millis(100)))?;
            let mut r = BufReader::new(s);
            let mut line = String::new();
            r.read_line(&mut line)?;
            r.get_mut().write_all(response)?;
            let mut next = String::new();
            match r.read_line(&mut next) {
                Ok(0) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                other => assert!(
                    false,
                    "unexpected request after uncertain failure: {other:?} {next}"
                ),
            }
            Ok(())
        });
        let mut c = Client::connect(port)?;
        if inspect {
            assert!(native::open_safely(&mut c, Path::new("missing.vectorcraft")).is_err());
        } else {
            assert!(c.request("test", json!({"text":"中文"})).is_err());
            assert!(c.request("mutate", json!({})).is_err());
        }
        worker
            .join()
            .map_err(|_| std::io::Error::other("worker panic"))??;
        Ok(())
    }
    #[test]
    fn real_stream_checks_reply_identity_and_preserves_unicode() -> TestResult {
        failed_exchange(b"{\"id\":99,\"ok\":true,\"result\":{}}\n", false)
    }
    #[test]
    fn inspect_failure_never_opens_document() -> TestResult {
        failed_exchange(b"{\"id\":99,\"ok\":true,\"result\":{}}\n", true)
    }
    #[test]
    fn timeout_poisons_transport_before_second_mutation() -> TestResult {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let worker = std::thread::spawn(move || -> std::io::Result<()> {
            let (s, _) = listener.accept()?;
            let mut r = BufReader::new(s);
            let mut line = String::new();
            r.read_line(&mut line)?;
            std::thread::sleep(Duration::from_millis(60));
            let mut next = String::new();
            assert_eq!(r.read_line(&mut next)?, 0);
            Ok(())
        });
        let mut c = Client::connect(port)?;
        c.stream
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(10)))?;
        assert!(c.request("test", json!({})).is_err());
        assert!(c.request("mutate", json!({})).is_err());
        worker
            .join()
            .map_err(|_| std::io::Error::other("worker panic"))??;
        Ok(())
    }

    #[test]
    fn resource_diagnostics_survive_same_path_followup() -> TestResult {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let worker = std::thread::spawn(move || -> std::io::Result<()> {
            let (s, _) = listener.accept()?;
            s.set_read_timeout(Some(Duration::from_millis(100)))?;
            let mut r = BufReader::new(s);
            for response in [
                json!({"path":"old.vectorcraft","dirty":false}),
                json!({"missingLinks":[{"path":"missing.png"}]}),
                json!({"path":"new.vectorcraft","dirty":false}),
            ] {
                let mut line = String::new();
                match r.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        break;
                    }
                    Err(e) => return Err(e),
                }
                let request: Value = serde_json::from_str(&line)?;
                r.get_mut().write_all(
                    format!(
                        "{}\n",
                        json!({"id":request["id"],"ok":true,"result":response})
                    )
                    .as_bytes(),
                )?;
            }
            drop(r);
            let (stream, _) = listener.accept()?;
            let mut next = BufReader::new(stream);
            let mut line = String::new();
            assert_eq!(
                next.read_line(&mut line)?,
                0,
                "fresh client must refuse cached diagnostics without sending a request"
            );
            Ok(())
        });
        let mut c = Client::connect(port)?;
        assert!(native::open_safely(&mut c, Path::new("new.vectorcraft")).is_err());
        assert!(
            native::open_safely(&mut c, Path::new("new.vectorcraft")).is_err(),
            "second operation must preserve missing-link refusal"
        );
        let diagnostics = c.diagnostics.clone();
        drop(c);
        let mut next = Client::connect(port)?;
        next.diagnostics = diagnostics;
        assert!(native::open_safely(&mut next, Path::new("new.vectorcraft")).is_err());
        drop(next);
        worker
            .join()
            .map_err(|_| std::io::Error::other("worker panic"))??;
        Ok(())
    }
}
