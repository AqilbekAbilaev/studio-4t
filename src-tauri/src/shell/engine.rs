// Embedded JavaScript shell ("IntelliShell").
//
// `boa_engine::Context` is `!Send`, so it cannot live in Tauri managed state or
// cross an `.await`. We confine every JS context to a single owned worker thread
// and talk to it over a channel. Each shell tab gets its own `Context` (keyed by
// session id) so that variables declared in one submission survive into the next.
//
// Phase 1: plain JavaScript only — `print` / `printjson` (defined in a JS
// preamble that appends to a `__logs__` array) plus the completion value of the
// evaluated code. The MongoDB `db` bridge is added in a later phase.

use std::collections::HashMap;

use boa_engine::{Context, Source};
use serde::Serialize;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

/// One evaluation's transcript, returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ShellResult {
    /// Lines emitted via `print` / `printjson`, in order.
    pub logs: Vec<String>,
    /// The completion value of the submitted code, as EJSON-shaped JSON.
    /// `None` when the code evaluates to `undefined`.
    pub value: Option<serde_json::Value>,
    /// A JavaScript runtime/parse error message, if the submission threw.
    /// Carried here (not as a command failure) so one bad line doesn't abort
    /// the whole console session.
    pub error: Option<String>,
}

/// Messages sent from Tauri commands to the worker thread.
enum ShellRequest {
    Eval {
        session_id: String,
        code: String,
        reply: oneshot::Sender<ShellResult>,
    },
    Close {
        session_id: String,
    },
}

/// Managed-state handle to the shell worker thread. Holds only the channel
/// sender, so it is `Send + Sync` and safe to `app.manage()`.
pub struct ShellEngine {
    sender: UnboundedSender<ShellRequest>,
}

impl ShellEngine {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded_channel::<ShellRequest>();
        std::thread::Builder::new()
            .name(String::from("intellishell"))
            .spawn(move || worker(receiver))
            .expect("failed to spawn IntelliShell worker thread");
        ShellEngine { sender: sender }
    }

    /// Queue `code` for evaluation in `session_id`'s context and return a
    /// receiver the async command can await for the transcript.
    pub fn submit_eval(&self, session_id: String, code: String) -> oneshot::Receiver<ShellResult> {
        let (reply, reply_rx) = oneshot::channel::<ShellResult>();
        let request = ShellRequest::Eval {
            session_id: session_id,
            code: code,
            reply: reply,
        };
        // If the worker is gone the request (and its reply sender) is dropped,
        // so the awaited receiver resolves to an error — handled by the caller.
        let _ = self.sender.send(request);
        reply_rx
    }

    /// Drop a session's context (e.g. when its shell tab is closed).
    pub fn close(&self, session_id: String) {
        let request = ShellRequest::Close {
            session_id: session_id,
        };
        let _ = self.sender.send(request);
    }
}

/// The worker thread: owns all JS contexts and serves requests one at a time.
/// Runs on a dedicated `std::thread` (not a runtime worker), so `blocking_recv`
/// is safe here.
fn worker(mut receiver: UnboundedReceiver<ShellRequest>) {
    let mut sessions: HashMap<String, Context> = HashMap::new();

    loop {
        let request = match receiver.blocking_recv() {
            Some(value) => value,
            None => break, // all senders dropped — app is shutting down
        };

        match request {
            ShellRequest::Eval {
                session_id,
                code,
                reply,
            } => {
                let context = sessions
                    .entry(session_id)
                    .or_insert_with(new_context);
                let result = eval_in(context, &code);
                let _ = reply.send(result);
            }
            ShellRequest::Close { session_id } => {
                sessions.remove(&session_id);
            }
        }
    }
}

/// Build a fresh JS context with hang limits and the `print` preamble installed.
fn new_context() -> Context {
    let mut context = Context::default();

    // Hang protection: abort runaway loops / recursion instead of freezing the
    // worker thread. Generous enough for legitimate iteration over results.
    context
        .runtime_limits_mut()
        .set_loop_iteration_limit(10_000_000);
    context.runtime_limits_mut().set_recursion_limit(2_000);

    // `print` / `printjson` collect into a global array we read back after each
    // submission. Defining them in JS avoids boa's GC-traceable native captures.
    let preamble = r#"
        globalThis.__logs__ = [];
        globalThis.print = function () {
            var parts = [];
            for (var i = 0; i < arguments.length; i++) {
                var a = arguments[i];
                parts.push(typeof a === 'string' ? a : JSON.stringify(a));
            }
            globalThis.__logs__.push(parts.join(' '));
        };
        globalThis.printjson = function (x) {
            globalThis.__logs__.push(JSON.stringify(x, null, 2));
        };
    "#;
    let _ = context.eval(Source::from_bytes(preamble));

    context
}

/// Reset the log buffer, run the user's code, and collect the transcript.
fn eval_in(context: &mut Context, code: &str) -> ShellResult {
    let _ = context.eval(Source::from_bytes("__logs__ = [];"));

    let outcome = context.eval(Source::from_bytes(code));
    let logs = read_logs(context);

    match outcome {
        Ok(value) => {
            let json = match value.to_json(context) {
                Ok(option) => option,
                Err(_) => None,
            };
            ShellResult {
                logs: logs,
                value: json,
                error: None,
            }
        }
        Err(err) => ShellResult {
            logs: logs,
            value: None,
            error: Some(err.to_string()),
        },
    }
}

/// Read the `__logs__` global back out as plain strings.
fn read_logs(context: &mut Context) -> Vec<String> {
    let value = match context.eval(Source::from_bytes("__logs__")) {
        Ok(val) => val,
        Err(_) => return Vec::new(),
    };
    let json = match value.to_json(context) {
        Ok(Some(serde_json::Value::Array(array))) => array,
        Ok(_) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    let mut logs = Vec::new();
    for item in json {
        match item {
            serde_json::Value::String(text) => logs.push(text),
            other => logs.push(other.to_string()),
        }
    }
    logs
}
