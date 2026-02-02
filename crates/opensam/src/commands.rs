//! OpenSAM command implementations

use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use opensam_agent::AgentLoop;
use opensam_bus::{InboundMessage, MessageBus, OutboundDispatcher};
use opensam_channels::{Channel, TelegramChannel};
use opensam_config::{self, Config, ProviderConfig, TelegramConfig};
use opensam_cron::{CronService, Job, Payload, Schedule};
use opensam_provider::openrouter::OpenRouterProvider;

/// Get path to cron job store
fn cron_store_path() -> std::path::PathBuf {
    opensam_config::data_dir()
        .join("timeline")
        .join("cron.json")
}

/// List scheduled jobs
pub async fn schedule_list_command(all: bool) -> Result<()> {
    let store_path = cron_store_path();
    let mut service = CronService::new(&store_path);
    service.load().await?;

    let jobs = service.list_jobs(all);

    if jobs.is_empty() {
        println!("No scheduled jobs");
    } else {
        println!("Scheduled jobs:");
        for job in jobs {
            let status = if job.enabled { "enabled" } else { "disabled" };
            println!(
                "  {} - {} ({}, {})",
                job.id,
                job.name,
                status,
                match &job.schedule {
                    Schedule::Every { every_ms } => format!("every {}s", every_ms / 1000),
                    Schedule::Cron { expr } => format!("cron: {}", expr),
                    Schedule::At { at_ms } => format!("at: {}", at_ms),
                }
            );
        }
    }

    Ok(())
}

/// Add a scheduled job
pub async fn schedule_add_command(
    name: String,
    message: String,
    every: Option<u64>,
    cron: Option<String>,
) -> Result<()> {
    let store_path = cron_store_path();
    let mut service = CronService::new(&store_path);
    service.load().await?;

    let schedule = if let Some(seconds) = every {
        Schedule::Every {
            every_ms: (seconds * 1000) as i64,
        }
    } else if let Some(expr) = cron {
        Schedule::Cron { expr }
    } else {
        anyhow::bail!("Either --every or --cron must be specified");
    };

    let payload = Payload::new(message);
    let job = Job::new(name, schedule, payload);

    service.add_job(job).await;
    service.save().await?;

    println!("✓ Job added");
    Ok(())
}

/// Remove a scheduled job
pub async fn schedule_remove_command(id: String) -> Result<()> {
    let store_path = cron_store_path();
    let mut service = CronService::new(&store_path);
    service.load().await?;

    if service.remove_job(&id).await {
        println!("✓ Job {} removed", id);
    } else {
        println!("✗ Job {} not found", id);
    }

    Ok(())
}

/// Show frequency/channel status
pub async fn freq_status_command() -> Result<()> {
    let config = Config::load().await?;

    println!("◆ Channel Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Telegram status
    let tg = &config.frequency.telegram;
    println!("Telegram:");
    println!("  Enabled: {}", if tg.enabled { "Yes" } else { "No" });
    println!(
        "  Token: {}",
        if tg.token.is_empty() {
            "[Not set]"
        } else {
            "[Set]"
        }
    );
    let allowed = if tg.allow_from.is_empty() {
        "Any".to_string()
    } else {
        tg.allow_from.join(", ")
    };
    println!("  Allowed users: {}", allowed);

    Ok(())
}

/// Read line from stdin
fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

/// Read password from stdin (masked input)
fn read_password() -> String {
    rpassword::read_password().unwrap_or_else(|_| read_line())
}

/// OpenRouter model response
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
    name: Option<String>,
}

/// Target models to look for
const TARGET_MODELS: &[&str] = &["kimi", "minimax", "gemini", "claude", "gpt"];

/// Fetch available models from OpenRouter
async fn fetch_openrouter_models(api_key: &str) -> Result<Vec<ModelInfo>> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch models: {}", response.status());
    }

    let models: ModelsResponse = response.json().await?;
    Ok(models.data)
}

/// Validate API key by making a test request
async fn validate_api_key(api_key: &str) -> bool {
    fetch_openrouter_models(api_key).await.is_ok()
}

/// Filter models to show only target ones
fn filter_models(models: Vec<ModelInfo>) -> Vec<ModelInfo> {
    models
        .into_iter()
        .filter(|m| {
            let id_lower = m.id.to_lowercase();
            TARGET_MODELS.iter().any(|target| id_lower.contains(target))
        })
        .collect()
}

/// Interactive setup wizard
pub async fn setup_command() -> Result<()> {
    println!("◆ OpenSAM Setup Wizard");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // ========================================================
    // Step 1: OpenRouter API Key
    // ========================================================
    println!("Step 1: OpenRouter API Key");
    println!("Get your API key at: https://openrouter.ai/keys");
    println!();

    let api_key = loop {
        print!("Enter your OpenRouter API key: ");
        std::io::stdout().flush()?;
        let key = read_password();

        if key.is_empty() {
            println!("API key cannot be empty. Please try again.");
            continue;
        }

        print!("Validating API key... ");
        std::io::stdout().flush()?;

        if validate_api_key(&key).await {
            println!("✓ Valid!");
            break key;
        } else {
            println!("✗ Invalid");
            println!();
            print!("The API key appears to be invalid. Try again? (Y/n/skip): ");
            std::io::stdout().flush()?;
            let response = read_line().to_lowercase();

            if response == "skip" || response == "s" {
                println!(
                    "Skipping API key validation. You can set it later in ~/.opensam/config.json"
                );
                break key;
            } else if response == "n" || response == "no" {
                anyhow::bail!("Setup cancelled");
            }
            // Otherwise, loop and try again
        }
    };
    println!();

    // ========================================================
    // Step 2: Select Model
    // ========================================================
    println!("Step 2: Select Default Model");
    println!();

    let model_id = if !api_key.is_empty() {
        print!("Fetching available models... ");
        std::io::stdout().flush()?;

        match fetch_openrouter_models(&api_key).await {
            Ok(models) => {
                let filtered = filter_models(models);
                println!("✓ Found {} models", filtered.len());
                println!();

                if !filtered.is_empty() {
                    println!("Available models:");
                    for (i, model) in filtered.iter().take(10).enumerate() {
                        let name = model.name.as_ref().unwrap_or(&model.id);
                        println!("  {}. {} ({})", i + 1, name, model.id);
                    }
                    println!();
                }

                println!("Options:");
                println!("  1-10. Select a model from the list above");
                println!("  m.    Enter model ID manually");
                println!("  d.    Use default (anthropic/claude-sonnet-4)");
                println!();
                print!("Your choice: ");
                std::io::stdout().flush()?;

                let choice = read_line();

                match choice.as_str() {
                    "m" | "M" => {
                        print!("Enter model ID (e.g., anthropic/claude-sonnet-4): ");
                        std::io::stdout().flush()?;
                        read_line()
                    }
                    "d" | "D" | "" => "anthropic/claude-sonnet-4".to_string(),
                    num => {
                        if let Ok(idx) = num.parse::<usize>() {
                            if idx > 0 && idx <= filtered.len() {
                                filtered[idx - 1].id.clone()
                            } else {
                                println!("Invalid selection, using default.");
                                "anthropic/claude-sonnet-4".to_string()
                            }
                        } else {
                            println!("Invalid input, using default.");
                            "anthropic/claude-sonnet-4".to_string()
                        }
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch models: {}", e);
                println!("Using default model.");
                "anthropic/claude-sonnet-4".to_string()
            }
        }
    } else {
        println!("Skipping model selection (no API key). Using default.");
        "anthropic/claude-sonnet-4".to_string()
    };
    println!();

    // ========================================================
    // Step 3: Telegram Settings
    // ========================================================
    println!("Step 3: Telegram Integration (Optional)");
    println!();
    print!("Enable Telegram bot? (y/N): ");
    std::io::stdout().flush()?;

    let enable_telegram = read_line().to_lowercase() == "y";

    let (tg_token, tg_allow_from) = if enable_telegram {
        print!("Enter Telegram bot token: ");
        std::io::stdout().flush()?;
        let token = read_password();

        print!("Enter allowed user IDs (comma-separated, empty for any): ");
        std::io::stdout().flush()?;
        let users_str = read_line();

        let allow_from: Vec<String> = users_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        (token, allow_from)
    } else {
        (String::new(), Vec::new())
    };
    println!();

    // ========================================================
    // Step 4: Save Configuration
    // ========================================================
    println!("Step 4: Saving Configuration");
    print!("Creating config... ");
    std::io::stdout().flush()?;

    // Load existing config or create new one
    let config_path = opensam_config::config_path();
    let mut config = if config_path.exists() {
        Config::load().await.unwrap_or_default()
    } else {
        Config::default()
    };

    // Update configuration
    config.providers.openrouter = ProviderConfig {
        api_key,
        api_base: Some("https://openrouter.ai/api/v1".to_string()),
    };
    config.operative.defaults.model = model_id;
    config.frequency.telegram = TelegramConfig {
        enabled: enable_telegram,
        token: tg_token,
        allow_from: tg_allow_from,
    };

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    config.save().await?;
    println!("✓ Saved to {}", config_path.display());
    println!();

    // ========================================================
    // Step 5: Initialize Workspace if needed
    // ========================================================
    println!("Step 5: Workspace Setup");

    let workspace = opensam_config::workspace_path();
    if !workspace.exists() {
        print!("Initializing workspace at {}... ", workspace.display());
        std::io::stdout().flush()?;

        tokio::fs::create_dir_all(&workspace).await?;
        tokio::fs::create_dir_all(workspace.join("lifepod")).await?;
        tokio::fs::create_dir_all(workspace.join("arsenal")).await?;

        create_template(&workspace, "DIRECTIVE.md", DIRECTIVE_MD).await?;
        create_template(&workspace, "PERSONA.md", PERSONA_MD).await?;
        create_template(&workspace, "SUBJECT.md", SUBJECT_MD).await?;
        create_template(&workspace.join("lifepod"), "MEMORY.md", MEMORY_MD).await?;

        println!("✓ Done");
    } else {
        println!("✓ Workspace already exists at {}", workspace.display());
    }
    println!();

    // ========================================================
    // Step 6: Ask to start gateway
    // ========================================================
    println!("Setup complete! ✓");
    println!();
    print!("Start gateway now on port 18789? (y/N): ");
    std::io::stdout().flush()?;

    let start_gateway = read_line().to_lowercase() == "y";

    if start_gateway {
        println!();
        deploy_command().await?;
    } else {
        println!();
        println!("You can start the gateway later with: sam deploy");
        println!();
        println!("Next steps:");
        println!("  - Chat with OpenSAM: sam engage -m \"Hello!\"");
        println!("  - Start gateway:      sam deploy");
        println!("  - Check status:       sam status");
    }

    Ok(())
}

/// Initialize config and workspace
pub async fn init_command() -> Result<()> {
    println!("◆ Initializing OpenSAM...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let config = opensam_config::init().await?;

    let workspace = config.workspace_path();
    tokio::fs::create_dir_all(&workspace).await?;
    tokio::fs::create_dir_all(workspace.join("lifepod")).await?;
    tokio::fs::create_dir_all(workspace.join("arsenal")).await?;

    create_template(&workspace, "DIRECTIVE.md", DIRECTIVE_MD).await?;
    create_template(&workspace, "PERSONA.md", PERSONA_MD).await?;
    create_template(&workspace, "SUBJECT.md", SUBJECT_MD).await?;
    create_template(&workspace.join("lifepod"), "MEMORY.md", MEMORY_MD).await?;

    println!("\n◆ OpenSAM initialized");
    println!("\nNext steps:");
    println!("  1. Add your API key to ~/.opensam/config.json");
    println!("     Get one at: https://openrouter.ai/keys");
    println!("  2. Start chatting: sam engage -m \"Hello!\"");

    Ok(())
}

async fn create_template(dir: &std::path::Path, filename: &str, content: &str) -> Result<()> {
    let path = dir.join(filename);
    if !path.exists() {
        tokio::fs::write(&path, content).await?;
        info!("◆ Created {}", path.display());
    }
    Ok(())
}

/// Chat with the agent
pub async fn engage_command(message: Option<String>, _session: String) -> Result<()> {
    let config = Config::load().await?;

    let api_key = config
        .api_key()
        .context("No API key configured. Set one in ~/.opensam/config.json")?;
    let api_base = config.api_base();
    let model = config.default_model();

    let provider = OpenRouterProvider::new(api_key, api_base, Some(model));
    let (bus, _in_rx, _out_rx) = MessageBus::channels();

    let agent = AgentLoop::with_config(
        bus.clone(),
        provider,
        config.workspace_path(),
        config.default_model(),
        20,
        config.brave_api_key(),
        &config,
    );

    if let Some(msg) = message {
        let inbound = InboundMessage::new("field", "user", "direct", msg);
        if let Some(response) = agent.process_message(inbound).await {
            println!("\n◆ {}", response.content);
        }
    } else {
        println!("◆ Interactive mode (type 'exit' to quit)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        loop {
            print!("◆ ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            let input = input.trim();
            if input.is_empty() {
                continue;
            }
            if input == "exit" || input == "quit" {
                break;
            }

            let inbound = InboundMessage::new("field", "user", "direct", input.to_string());
            if let Some(response) = agent.process_message(inbound).await {
                println!("\n◆ {}\n", response.content);
            }
        }
    }

    Ok(())
}

/// Start gateway server
pub async fn deploy_command() -> Result<()> {
    // Telemetry: Track start time and message count
    let start_time = std::time::Instant::now();
    let message_count = Arc::new(AtomicU64::new(0));
    let message_count_for_inbound = Arc::clone(&message_count);

    println!("◆ Starting OpenSAM gateway");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let config = Config::load().await?;

    // Telemetry: Log enabled channels
    info!(
        "Channels enabled: telegram={}",
        config.frequency.telegram.enabled
    );
    debug!("Session max_messages: {}", config.session_max_messages());
    debug!(
        "Web search max_results: {}",
        config.web_search_max_results()
    );

    let api_key = config.api_key().context("No API key configured")?;
    let api_base = config.api_base();

    let provider = OpenRouterProvider::new(api_key, api_base, Some(config.default_model()));
    let (bus, mut in_rx, out_rx) = MessageBus::channels();

    let agent = AgentLoop::with_config(
        bus.clone(),
        provider,
        config.workspace_path(),
        config.default_model(),
        20,
        config.brave_api_key(),
        &config,
    );

    // Create channel for coordinating shutdown
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    // ========================================
    // 1. Initialize Telegram channel if enabled
    // ========================================
    let telegram_channel: Option<TelegramChannel> = if config.frequency.telegram.enabled {
        let tg_config = opensam_channels::telegram::TelegramConfig {
            enabled: config.frequency.telegram.enabled,
            token: config.frequency.telegram.token.clone(),
            allow_from: config.frequency.telegram.allow_from.clone(),
        };
        info!("◆ Initializing Telegram channel");
        Some(TelegramChannel::new(tg_config, bus.clone()))
    } else {
        info!("◆ Telegram channel disabled");
        None
    };

    // Spawn channel tasks
    let mut channel_handles = vec![];

    if let Some(mut channel) = telegram_channel {
        let channel_task = tokio::spawn(async move {
            info!("◆ Starting Telegram channel task");
            if let Err(e) = channel.start().await {
                error!("Telegram channel error: {}", e);
            }
            info!("◆ Telegram channel stopped");
        });
        channel_handles.push(channel_task);
    }

    // ========================================
    // 2. Inbound processing loop
    // ========================================
    let agent_for_inbound = agent;
    let bus_for_inbound = bus.clone();

    let inbound_task = tokio::spawn(async move {
        info!("◆ Inbound processing loop started");

        loop {
            tokio::select! {
                msg = in_rx.recv() => {
                    match msg {
                        Some(inbound) => {
                            // Telemetry: Track message count
                            message_count_for_inbound.fetch_add(1, Ordering::SeqCst);

                            debug!("Processing inbound message from {}", inbound.sender_id);

                            // Process the message through the agent
                            match agent_for_inbound.process_message(inbound.clone()).await {
                                Some(response) => {
                                    // Publish the response to outbound queue
                                    if let Err(e) = bus_for_inbound.publish_outbound(response) {
                                        error!("Failed to publish outbound message: {}", e);
                                    }
                                }
                                None => {
                                    debug!("No response from agent for message from {}", inbound.sender_id);
                                }
                            }
                        }
                        None => {
                            info!("◆ Inbound channel closed, shutting down processing loop");
                            break;
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("◆ Shutdown signal received in inbound loop");
                    break;
                }
            }
        }

        info!("◆ Inbound processing loop stopped");
    });

    // ========================================
    // 3. Outbound dispatcher
    // ========================================
    let mut dispatcher = OutboundDispatcher::new(out_rx);

    // Register Telegram handler if enabled
    if config.frequency.telegram.enabled && !config.frequency.telegram.token.is_empty() {
        let tg_config = opensam_channels::telegram::TelegramConfig {
            enabled: config.frequency.telegram.enabled,
            token: config.frequency.telegram.token.clone(),
            allow_from: config.frequency.telegram.allow_from.clone(),
        };

        dispatcher.on_channel("telegram", move |msg| {
            let tg_config = tg_config.clone();
            tokio::spawn(async move {
                let bus = MessageBus::new(
                    tokio::sync::mpsc::unbounded_channel().0,
                    tokio::sync::mpsc::unbounded_channel().0,
                );
                let channel = TelegramChannel::new(tg_config, bus);
                if let Err(e) = channel.send(&msg).await {
                    error!("Failed to send message via Telegram: {}", e);
                }
            });
        });
    }

    let dispatcher_task = tokio::spawn(async move {
        info!("◆ Outbound dispatcher started");
        dispatcher.run().await;
        info!("◆ Outbound dispatcher stopped");
    });

    // ========================================
    // 4. Main service loop with graceful shutdown
    // ========================================
    info!("◆ Gateway active");
    println!("◆ Gateway active");
    println!("Channels: telegram={}", config.frequency.telegram.enabled);
    println!("Waiting for connections...");
    println!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("◆ Shutdown signal received, initiating graceful shutdown...");
            println!("\n◆ Shutting down...");
        }
        _ = shutdown_rx.recv() => {
            info!("◆ Shutdown requested via channel");
        }
    }

    // ========================================
    // 5. Cleanup: Drop channels to signal tasks to stop
    // ========================================
    info!("◆ Signaling tasks to stop...");
    drop(shutdown_tx);

    // Drop the bus to signal channel tasks
    drop(bus);

    // Wait for all tasks to complete (with timeout)
    let shutdown_timeout = tokio::time::Duration::from_secs(5);

    info!("◆ Waiting for tasks to complete...");

    // Wait for inbound task
    match tokio::time::timeout(shutdown_timeout, inbound_task).await {
        Ok(Ok(())) => info!("◆ Inbound task completed gracefully"),
        Ok(Err(e)) => warn!("◆ Inbound task panicked: {}", e),
        Err(_) => warn!("◆ Inbound task shutdown timed out"),
    }

    // Wait for dispatcher task
    match tokio::time::timeout(shutdown_timeout, dispatcher_task).await {
        Ok(Ok(())) => info!("◆ Dispatcher task completed gracefully"),
        Ok(Err(e)) => warn!("◆ Dispatcher task panicked: {}", e),
        Err(_) => warn!("◆ Dispatcher task shutdown timed out"),
    }

    // Wait for channel tasks
    for (i, handle) in channel_handles.into_iter().enumerate() {
        match tokio::time::timeout(shutdown_timeout, handle).await {
            Ok(Ok(())) => info!("◆ Channel task {} completed gracefully", i),
            Ok(Err(e)) => warn!("◆ Channel task {} panicked: {}", i, e),
            Err(_) => warn!("◆ Channel task {} shutdown timed out", i),
        }
    }

    // Telemetry: Calculate uptime and log summary
    let elapsed = start_time.elapsed();
    let processed = message_count.load(Ordering::SeqCst);

    info!(
        "◆ Gateway ran for {:?}, processed {} messages",
        elapsed, processed
    );
    println!("◆ Gateway ran for {:?}", elapsed);
    println!("◆ Processed {} messages", processed);
    println!("◆ Gateway shutdown complete");

    Ok(())
}

/// Show status
pub async fn status_command() -> Result<()> {
    let config_path = opensam_config::config_path();
    let workspace = opensam_config::workspace_path();

    println!("◆ OpenSAM System Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!(
        "Config:   {} {}",
        config_path.display(),
        if config_path.exists() {
            "[OK]"
        } else {
            "[Missing]"
        }
    );
    println!(
        "Workspace: {} {}",
        workspace.display(),
        if workspace.exists() {
            "[OK]"
        } else {
            "[Missing]"
        }
    );

    if config_path.exists() {
        let config = Config::load().await?;
        println!("Model:     {}", config.default_model());
        println!(
            "API Key:   {}",
            if config.has_api_key() {
                "[Set]"
            } else {
                "[Missing]"
            }
        );
        println!(
            "Telegram:  {}",
            if config.frequency.telegram.enabled {
                "[Enabled]"
            } else {
                "[Disabled]"
            }
        );
        println!("Session max: {} messages", config.session_max_messages());
    }

    println!("\n◆ Ready");

    Ok(())
}

// Template content
const DIRECTIVE_MD: &str = r#"# Agent Directives

You are a helpful AI assistant running in a terminal. Be concise and practical.

## Guidelines

- Explain what you're doing before taking actions
- Ask for clarification when requests are ambiguous
- Use tools to accomplish tasks efficiently
- Remember important information in memory files
"#;

const PERSONA_MD: &str = r#"# Persona

Name: OpenSAM
Type: Terminal AI Agent

## Traits

- Helpful and direct
- Concise, no fluff
- Technically competent
- Occasional dry wit

## Communication Style

Keep it practical. Code blocks are good. Bullet points over paragraphs when possible.
"#;

const SUBJECT_MD: &str = r#"# User Profile

Information about the user.

## Preferences

- Style: [concise/detailed]
- TZ: [timezone]
- Lang: [language]
"#;

const MEMORY_MD: &str = r#"# Long-term Memory

Important information that persists across sessions.

## Facts

(Key facts about the user)

## Preferences

(Learned preferences)

## Notes

(Important things to remember)
"#;
