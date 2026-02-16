use std::sync::Arc;

use async_trait::async_trait;

use crate::error::LlmError;

use super::provider::{
    CompletionRequest, CompletionResponse, LlmProvider, ModelMetadata, ToolCompletionRequest,
    ToolCompletionResponse,
};

/// OpenAI-specific wrapper that applies schema compatibility transforms for GPT-5 models.
pub struct OpenAiToolSchemaProvider {
    inner: Arc<dyn LlmProvider>,
}

impl OpenAiToolSchemaProvider {
    pub fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self { inner }
    }

    fn requires_strict_required(model_name: &str) -> bool {
        model_name.to_lowercase().starts_with("gpt-5")
    }

    fn normalize_required_fields(schema: &serde_json::Value) -> serde_json::Value {
        match schema {
            serde_json::Value::Object(map) => {
                let mut normalized = serde_json::Map::new();

                for (key, value) in map {
                    normalized.insert(key.clone(), Self::normalize_required_fields(value));
                }

                if map.get("type") == Some(&serde_json::Value::String("object".to_string()))
                    && let Some(serde_json::Value::Object(properties)) = map.get("properties")
                {
                    let required = properties
                        .keys()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect::<Vec<_>>();
                    normalized.insert("required".to_string(), serde_json::Value::Array(required));
                }

                serde_json::Value::Object(normalized)
            }
            serde_json::Value::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(Self::normalize_required_fields)
                    .collect::<Vec<_>>(),
            ),
            _ => schema.clone(),
        }
    }

    fn normalize_tool_request(&self, mut request: ToolCompletionRequest) -> ToolCompletionRequest {
        if !Self::requires_strict_required(&self.inner.active_model_name()) {
            return request;
        }

        for tool in &mut request.tools {
            tool.parameters = Self::normalize_required_fields(&tool.parameters);
        }

        request
    }
}

#[async_trait]
impl LlmProvider for OpenAiToolSchemaProvider {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (rust_decimal::Decimal, rust_decimal::Decimal) {
        self.inner.cost_per_token()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.complete(request).await
    }

    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        let request = self.normalize_tool_request(request);
        self.inner.complete_with_tools(request).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        self.inner.model_metadata().await
    }

    fn active_model_name(&self) -> String {
        self.inner.active_model_name()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        self.inner.set_model(model)
    }

    fn seed_response_chain(&self, thread_id: &str, response_id: String) {
        self.inner.seed_response_chain(thread_id, response_id)
    }

    fn get_response_chain_id(&self, thread_id: &str) -> Option<String> {
        self.inner.get_response_chain_id(thread_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_strict_required() {
        assert!(OpenAiToolSchemaProvider::requires_strict_required("gpt-5"));
        assert!(OpenAiToolSchemaProvider::requires_strict_required(
            "GPT-5.2"
        ));
        assert!(!OpenAiToolSchemaProvider::requires_strict_required(
            "gpt-4o"
        ));
    }

    #[test]
    fn test_normalize_required_fields_for_object_nodes() {
        let input = serde_json::json!({
            "type": "object",
            "properties": {
                "method": {"type": "string"},
                "url": {"type": "string"},
                "body": {
                    "type": "object",
                    "properties": {
                        "q": {"type": "string"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["q"]
                }
            },
            "required": ["method", "url"]
        });

        let normalized = OpenAiToolSchemaProvider::normalize_required_fields(&input);

        let required = normalized["required"].as_array().expect("required array");
        assert_eq!(required.len(), 3);
        assert!(required.contains(&serde_json::Value::String("method".into())));
        assert!(required.contains(&serde_json::Value::String("url".into())));
        assert!(required.contains(&serde_json::Value::String("body".into())));

        let nested_required = normalized["properties"]["body"]["required"]
            .as_array()
            .expect("nested required array");
        assert_eq!(nested_required.len(), 2);
        assert!(nested_required.contains(&serde_json::Value::String("q".into())));
        assert!(nested_required.contains(&serde_json::Value::String("limit".into())));
    }
}
