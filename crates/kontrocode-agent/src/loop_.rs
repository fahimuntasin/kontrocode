//! The agent loop — the brain.
//!
//! Observes a user message, plans a response, optionally does research,
//! dispatches tool calls, and streams the final answer.

use std::path::PathBuf;
use std::sync::Arc;

use kontrocode_core::{analyze, CompletionRequest, Intent, Message, MessageId, ModelId, ToolCall};
use kontrocode_memory::MemoryStore;
use kontrocode_research::ResearchRunner;
use kontrocode_router::{Router, RouterEvent, TaskComplexity};
use tracing::{debug, warn};

use crate::prompt::{render_memory, with_memory, SYSTEM_PROMPT};
use crate::stream::{channel, AgentEvent, StreamReceiver, StreamSender};
use crate::tools::{dispatch, Tool};

/// Configuration for [`Agent`].
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// The project root. All file tools are resolved against this.
    pub project_root: PathBuf,
    /// Default model to use when the router doesn't pick one.
    pub default_model: ModelId,
    /// Maximum number of self-correction attempts.
    pub max_self_corrections: u32,
    /// Whether to run research before generating.
    pub enable_research: bool,
}

impl AgentConfig {
    /// Construct a default config rooted at `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            default_model: ModelId::new("mock", "echo"),
            max_self_corrections: 3,
            enable_research: true,
        }
    }
}

/// The result of a single agent run.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// The full message thread (system + user + assistant).
    pub messages: Vec<Message>,
    /// The final assistant message id.
    pub final_message_id: MessageId,
    /// The intent the analyzer detected.
    pub intent: Intent,
}

/// The agent. Cheap to clone.
#[derive(Clone)]
pub struct Agent {
    inner: Arc<AgentInner>,
}

struct AgentInner {
    config: AgentConfig,
    router: Router,
    runner: ResearchRunner,
    memory: Arc<dyn MemoryStore>,
    tools: Vec<Arc<dyn Tool>>,
    system_prompt: String,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("config", &self.inner.config)
            .field(
                "tools",
                &self
                    .inner
                    .tools
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Agent {
    /// Construct an agent. `tools` is the set of tools the model can call.
    pub fn new(
        config: AgentConfig,
        router: Router,
        runner: ResearchRunner,
        memory: Arc<dyn MemoryStore>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            inner: Arc::new(AgentInner {
                config,
                router,
                runner,
                memory,
                tools,
                system_prompt: SYSTEM_PROMPT.to_string(),
            }),
        }
    }

    /// Inject a custom system prompt. Defaults to the PRD §11 prompt.
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        let inner = Arc::get_mut(&mut self.inner).expect("agent already shared");
        inner.system_prompt = prompt;
        self
    }

    /// Access the underlying router.
    pub fn router(&self) -> &Router {
        &self.inner.router
    }

    /// Access the underlying memory store.
    pub fn memory(&self) -> &Arc<dyn MemoryStore> {
        &self.inner.memory
    }

    /// Access the underlying research runner.
    pub fn runner(&self) -> &ResearchRunner {
        &self.inner.runner
    }

    /// The default model the agent uses.
    pub fn default_model(&self) -> &kontrocode_core::ModelId {
        &self.inner.config.default_model
    }

    /// Number of providers in the router's registry.
    pub fn registry_len(&self) -> usize {
        self.inner.router.registry().len()
    }

    /// Current project root.
    pub fn project_root(&self) -> &std::path::Path {
        &self.inner.config.project_root
    }

    /// Process a user message. Returns the [`StreamReceiver`] that the
    /// UI can listen on for [`AgentEvent`]s, plus a [`AgentOutput`]
    /// future that resolves when the run is complete.
    ///
    /// In Phase 1 the model is the mock provider, so no real tool calls
    /// are emitted; the agent runs the full protocol (intent → research →
    /// request) but always falls through to the model's text response.
    pub fn submit(
        &self,
        user_input: impl Into<String>,
    ) -> (
        StreamSender,
        StreamReceiver,
        tokio::task::JoinHandle<kontrocode_core::Result<AgentOutput>>,
    ) {
        let user_input = user_input.into();
        let (tx, rx) = channel();
        let this = self.clone();
        let tx2 = tx.clone();
        let handle = tokio::spawn(async move { this.run(user_input, tx2).await });
        (tx, rx, handle)
    }

    async fn run(
        &self,
        user_input: String,
        tx: StreamSender,
    ) -> kontrocode_core::Result<AgentOutput> {
        let user_message = Message::user(&user_input);
        let message_id = user_message.id;
        let _ = tx.send(AgentEvent::Started { message_id });

        // 1. Intent analysis (synchronous, lexical).
        let intent = analyze(&user_input);
        debug!(?intent, "intent analyzed");

        // 2. Memory lookup.
        let profile = self.inner.memory.load().await?;
        let memory_xml = render_memory(&profile);
        let system_with_memory = with_memory(&memory_xml);

        // 3. Research (if enabled and needed).
        if self.inner.config.enable_research && intent.needs_research {
            let topic = build_research_topic(&intent);
            let stack = intent
                .stacks
                .first()
                .copied()
                .unwrap_or(kontrocode_core::Stack::Unknown);
            let _ = tx.send(AgentEvent::ResearchUpdate {
                title: format!("Researching {}", stack.display_name()),
                body: topic.clone(),
            });
            match self.inner.runner.research(stack, &topic).await {
                Ok(report) => {
                    let _ = tx.send(AgentEvent::ResearchUpdate {
                        title: "Research complete".into(),
                        body: format!(
                            "Confidence: {:.0}%. {}",
                            report.confidence * 100.0,
                            report.notes.join(" ")
                        ),
                    });
                }
                Err(e) => {
                    warn!(error = %e, "research failed (continuing without)");
                }
            }
        }

        // 4. Build the request.
        let tool_defs: Vec<kontrocode_core::ToolDefinition> =
            self.inner.tools.iter().map(|t| t.definition()).collect();
        let mut req = CompletionRequest::new(
            self.inner.config.default_model.clone(),
            vec![Message::system(&system_with_memory), user_message.clone()],
        );
        req.tools = tool_defs;

        // 5. Route and call.
        let complexity = TaskComplexity::from_score(intent.complexity);
        let router = self.inner.router.clone();
        let tx_clone = tx.clone();
        let final_id = MessageId::new();
        let response = router
            .complete(complexity, req, |event| match event {
                RouterEvent::Selected { model, .. } => {
                    debug!(%model, "router selected model");
                }
                RouterEvent::Fallback { from, to, reason } => {
                    let _ = tx_clone.send(AgentEvent::ResearchUpdate {
                        title: "Provider fallback".into(),
                        body: format!("{from} → {to} ({reason})"),
                    });
                }
                RouterEvent::Exhausted { reason } => {
                    let _ = tx_clone.send(AgentEvent::Error {
                        message: format!("All providers failed: {reason}"),
                    });
                }
            })
            .await?;

        // 6. Stream the response to the UI.
        let assistant_message = response.message;
        let _ = tx.send(AgentEvent::TextChunk {
            message_id: final_id,
            delta: assistant_message.content.clone(),
        });

        // 7. Handle any tool calls (mock provider never emits them, but
        //    the wiring is real for Phase 2+).
        let mut all_messages = vec![Message::system(&system_with_memory), user_message.clone()];
        let mut to_call: Vec<ToolCall> = assistant_message.tool_calls.clone();
        let mut _attempts = 0u32;
        while !to_call.is_empty() && _attempts < self.inner.config.max_self_corrections {
            _attempts += 1;
            for call in &to_call {
                let _ = tx.send(AgentEvent::ToolCall {
                    message_id: final_id,
                    call: call.clone(),
                });
                let result = dispatch(&self.inner.tools, call).await;
                match result {
                    Ok(r) => {
                        let tool_msg =
                            Message::tool_result(&r.tool_call_id, r.output.as_message_string());
                        all_messages.push(tool_msg.clone());
                        let _ = tx.send(AgentEvent::ToolResult {
                            tool_call_id: r.tool_call_id.clone(),
                            result: r,
                        });
                    }
                    Err(e) => {
                        let err_msg = Message::tool_result(&call.id, format!("error: {e}"));
                        all_messages.push(err_msg);
                        let _ = tx.send(AgentEvent::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }
            // Real loop would re-call the model with tool results here.
            // Phase 1: just break, since the mock never produces tool calls.
            to_call.clear();
        }

        all_messages.push(assistant_message.clone());
        let _ = tx.send(AgentEvent::Done {
            message_id: final_id,
        });

        Ok(AgentOutput {
            messages: all_messages,
            final_message_id: final_id,
            intent,
        })
    }
}

fn build_research_topic(intent: &Intent) -> String {
    if intent.summary.is_empty() {
        format!("best practices for {:?}", intent.task_type)
    } else {
        intent.summary.clone()
    }
}
