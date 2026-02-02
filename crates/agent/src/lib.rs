//! OPERATIVE: Field Agent Core
//!
//! Tactical AI operative system with TOOLKIT deployment.

use thiserror::Error;

pub mod context;
pub mod loop_agent;
pub mod subagent;
pub mod tools;

pub use context::ContextBuilder;
pub use loop_agent::AgentLoop;
pub use subagent::SubagentManager;
pub use tools::{ToolRegistry, ToolTrait};

/// Operative errors
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("◆ TOOLKIT NOT FOUND: {0}")]
    ToolNotFound(String),

    #[error("◆ DEPLOYMENT FAILED: {0}")]
    ToolExecution(String),

    #[error("◆ DATA LINK ERROR: {0}")]
    Io(#[from] std::io::Error),

    #[error("◆ SOLITON ERROR: {0}")]
    Provider(String),

    #[error("◆ MAX ITERATIONS EXCEEDED")]
    MaxIterations,
}

pub type Result<T> = std::result::Result<T, AgentError>;
