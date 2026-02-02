//! Agent loop - core processing engine

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use opensam_bus::{InboundMessage, MessageBus, OutboundMessage};
use opensam_config::Config;
use opensam_provider::{ChatParams, Message, Provider, ToolCallDef, ToolChoice};
use opensam_session::SessionManager;

use crate::context::ContextBuilder;
use crate::tools::{self, MessageTool, ToolRegistry};

/// The agent loop processes messages and handles tool calls
#[allow(dead_code)]
pub struct AgentLoop<P: Provider> {
    bus: MessageBus,
    provider: Arc<P>,
    workspace: PathBuf,
    model: String,
    max_iterations: u32,
    brave_api_key: Option<String>,
    context: ContextBuilder,
    tools: ToolRegistry,
    session_manager: Arc<Mutex<SessionManager>>,
    max_history_messages: usize,
    message_tool: Arc<MessageTool>,
}

impl<P: Provider> AgentLoop<P> {
    /// Create a new agent loop
    pub fn new(
        bus: MessageBus,
        provider: P,
        workspace: PathBuf,
        model: String,
        max_iterations: u32,
        brave_api_key: Option<String>,
    ) -> Self {
        let config = Config::default();
        Self::with_config(
            bus,
            provider,
            workspace,
            model,
            max_iterations,
            brave_api_key,
            &config,
        )
    }

    /// Create a new agent loop with full configuration
    pub fn with_config(
        bus: MessageBus,
        provider: P,
        workspace: PathBuf,
        model: String,
        max_iterations: u32,
        brave_api_key: Option<String>,
        config: &Config,
    ) -> Self {
        let context = ContextBuilder::new(&workspace);
        let mut tools = ToolRegistry::new();
        let message_tool =
            Self::register_default_tools(&mut tools, config, &workspace, bus.clone());

        // Initialize session manager with max_messages from config
        let sessions_dir = dirs::home_dir()
            .map(|h| h.join(".opensam").join("ops").join("logs"))
            .unwrap_or_else(|| PathBuf::from(".opensam").join("ops").join("logs"));

        let max_messages = config.session_max_messages();
        let session_manager = Arc::new(Mutex::new(SessionManager::with_max_messages(
            sessions_dir,
            max_messages,
        )));

        Self {
            bus,
            provider: Arc::new(provider),
            workspace,
            model,
            max_iterations,
            brave_api_key,
            context,
            tools,
            session_manager,
            max_history_messages: 20, // Default: keep last 20 messages
            message_tool,
        }
    }

    /// Create a new agent loop with custom sessions directory (for testing)
    pub fn new_with_sessions_dir(
        bus: MessageBus,
        provider: P,
        workspace: PathBuf,
        model: String,
        max_iterations: u32,
        brave_api_key: Option<String>,
        sessions_dir: PathBuf,
    ) -> Self {
        let config = Config::default();
        Self::with_config_and_sessions_dir(
            bus,
            provider,
            workspace,
            model,
            max_iterations,
            brave_api_key,
            &config,
            sessions_dir,
        )
    }

    /// Create with config and custom sessions directory
    #[allow(clippy::too_many_arguments)]
    pub fn with_config_and_sessions_dir(
        bus: MessageBus,
        provider: P,
        workspace: PathBuf,
        model: String,
        max_iterations: u32,
        brave_api_key: Option<String>,
        config: &Config,
        sessions_dir: PathBuf,
    ) -> Self {
        let context = ContextBuilder::new(&workspace);
        let mut tools = ToolRegistry::new();
        let message_tool =
            Self::register_default_tools(&mut tools, config, &workspace, bus.clone());

        let max_messages = config.session_max_messages();
        let session_manager = Arc::new(Mutex::new(SessionManager::with_max_messages(
            sessions_dir,
            max_messages,
        )));

        Self {
            bus,
            provider: Arc::new(provider),
            workspace,
            model,
            max_iterations,
            brave_api_key,
            context,
            tools,
            session_manager,
            max_history_messages: 20,
            message_tool,
        }
    }

    /// Set the maximum number of history messages to keep
    pub fn set_max_history_messages(&mut self, max: usize) {
        self.max_history_messages = max;
    }

    /// Generate a session key from an inbound message
    /// Format: {channel}:{chat_id}
    pub fn generate_session_key(msg: &InboundMessage) -> String {
        format!("{}:{}", msg.channel, msg.chat_id)
    }

    fn register_default_tools(
        registry: &mut ToolRegistry,
        config: &Config,
        workspace: &std::path::Path,
        bus: MessageBus,
    ) -> Arc<MessageTool> {
        // Filesystem tools - with workspace
        registry.register(tools::ReadFileTool::new(workspace.to_path_buf()));
        registry.register(tools::WriteFileTool::new(workspace.to_path_buf()));
        registry.register(tools::EditFileTool::new(workspace.to_path_buf()));
        registry.register(tools::ListDirTool::new(workspace.to_path_buf()));

        // Shell tool - with workspace
        registry.register(tools::ExecTool::with_workspace(workspace.to_path_buf()));

        // Web tools - use config for max_results
        registry.register(tools::WebSearchTool::from_config(config));
        registry.register(tools::WebFetchTool::default());

        // Message tool - create with real outbound sender from the bus
        let sender = bus.outbound_sender();
        let message_tool = Arc::new(MessageTool::new(sender));
        registry.register((*message_tool).clone());

        message_tool
    }

    /// Process a message directly (for CLI)
    pub async fn process_direct(&self, content: &str, session_key: &str) -> String {
        let msg = InboundMessage::new("cli", "user", session_key, content);
        match self.process_message(msg).await {
            Some(response) => response.content,
            None => "No response".to_string(),
        }
    }

    /// Process a single message
    pub async fn process_message(&self, msg: InboundMessage) -> Option<OutboundMessage> {
        info!("Processing message from {}:{}", msg.channel, msg.sender_id);
        debug!("Content: {}", &msg.content[..msg.content.len().min(100)]);

        // Set context for message tool so it knows the current channel/chat_id
        self.message_tool
            .set_context(msg.channel.clone(), msg.chat_id.clone());

        // Generate session key from the message
        let session_key = Self::generate_session_key(&msg);

        // Load or create session and get history
        let history = {
            let mut session_manager = self.session_manager.lock().await;
            let session = session_manager.get_or_create(&session_key).await;
            session.get_history(self.max_history_messages)
        };

        // Build messages with history: system prompt + history + current message
        let messages = self.context.build_messages(history, &msg.content).await;

        // Run agent loop
        match self.run_agent_loop(messages).await {
            Ok(content) => {
                // Save session in a separate scope
                {
                    let mut session_manager = self.session_manager.lock().await;
                    let session = session_manager.get_or_create(&session_key).await;

                    // Append user message to session
                    session.add_message("user", &msg.content);

                    // Append assistant response to session
                    session.add_message("assistant", &content);

                    // Clone the session to save it
                    let session_clone = session.clone();
                    let _ = session; // Release mutable borrow

                    if let Err(e) = session_manager.save(&session_clone).await {
                        warn!("Failed to save session {}: {}", session_key, e);
                    }
                }

                Some(OutboundMessage::new(&msg.channel, &msg.chat_id, content))
            }
            Err(e) => {
                error!("Agent loop error: {}", e);

                // Even on error, try to save the user message
                {
                    let mut session_manager = self.session_manager.lock().await;
                    let session = session_manager.get_or_create(&session_key).await;
                    session.add_message("user", &msg.content);
                    session.add_message("assistant", format!("Error: {}", e));

                    let session_clone = session.clone();
                    let _ = session; // Release mutable borrow

                    if let Err(save_err) = session_manager.save(&session_clone).await {
                        warn!("Failed to save session {}: {}", session_key, save_err);
                    }
                }

                Some(OutboundMessage::new(
                    &msg.channel,
                    &msg.chat_id,
                    format!("Error: {}", e),
                ))
            }
        }
    }

    /// Run the agent loop with tool calling
    async fn run_agent_loop(&self, mut messages: Vec<Message>) -> crate::Result<String> {
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > self.max_iterations {
                return Err(crate::AgentError::MaxIterations);
            }

            debug!("Agent iteration {}", iteration);

            // Call LLM
            let params = ChatParams {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: self.tools.definitions(),
                tool_choice: ToolChoice::Auto,
                ..Default::default()
            };

            let response = self
                .provider
                .chat(params)
                .await
                .map_err(|e| crate::AgentError::Provider(e.to_string()))?;

            // Handle tool calls
            if response.has_tool_calls() {
                // Add assistant message with tool calls
                let tool_call_defs: Vec<ToolCallDef> = response
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCallDef::new(&tc.id, &tc.name, tc.arguments.clone()))
                    .collect();

                ContextBuilder::add_assistant_message(
                    &mut messages,
                    response.content.as_deref(),
                    Some(tool_call_defs),
                );

                // Execute tools
                for tool_call in &response.tool_calls {
                    debug!("Executing tool: {}", tool_call.name);

                    let result = self
                        .tools
                        .execute(&tool_call.name, tool_call.arguments.clone())
                        .await
                        .unwrap_or_else(|e| format!("Error: {}", e));

                    ContextBuilder::add_tool_result(
                        &mut messages,
                        &tool_call.id,
                        &tool_call.name,
                        &result,
                    );
                }
            } else {
                // No tool calls, return the content
                return Ok(response
                    .content
                    .unwrap_or_else(|| "Task completed.".to_string()));
            }
        }
    }
}
