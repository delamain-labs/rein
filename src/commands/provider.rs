/// Resolve a provider from an agent's model field.
///
/// Returns the provider on success or an exit code on failure.
pub fn resolve(
    agent: &rein::ast::AgentDef,
) -> Result<Box<dyn rein::runtime::provider::Provider>, i32> {
    let model_field = agent
        .model
        .as_ref()
        .map_or("openai".to_string(), format_value_expr);

    let config = rein::runtime::provider::resolver::ProviderConfig {
        openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
        openai_base_url: std::env::var("OPENAI_BASE_URL").ok(),
        anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        anthropic_base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
    };

    rein::runtime::provider::resolver::resolve(&model_field, &config).map_err(|e| {
        eprintln!("error: {e}");
        eprintln!("hint: set OPENAI_API_KEY or ANTHROPIC_API_KEY, or use --demo");
        1
    })
}

pub fn format_value_expr(v: &rein::ast::ValueExpr) -> String {
    match v {
        rein::ast::ValueExpr::Literal(s) => s.clone(),
        rein::ast::ValueExpr::EnvRef {
            var_name, default, ..
        } => match default {
            Some(d) => format!("env(\"{var_name}\", \"{d}\")"),
            None => format!("env(\"{var_name}\")"),
        },
    }
}

/// Block on a future, handling both inside and outside a tokio runtime.
pub fn block_on<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let handle = tokio::runtime::Handle::try_current();
    if let Ok(handle) = handle {
        tokio::task::block_in_place(|| handle.block_on(fut))
    } else {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(fut)
    }
}
