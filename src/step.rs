use anyhow::{Context, Result};

/// Run an async step, logging its name before execution and enriching any error with context.
pub async fn run_step<T>(
    name: &str,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tracing::info!("{}", name);
    fut.await.with_context(|| format!("{} failed", name))
}
