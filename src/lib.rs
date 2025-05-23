use futures::{StreamExt, future::BoxFuture};
use pb_rules_sdk::{
    futures::{ByteStreamWrapper, FutureCompat2},
    rules::Rule,
};

struct StdRules;

impl pb_rules_sdk::resolver::Resolver for StdRules {
    fn additional_interest_glob() -> Option<String> {
        tracing::info!("logging from additional interests glob");
        Some("pb.toml".into())
    }

    fn resolve_target(
        file: Vec<u8>,
    ) -> Result<Vec<pb_rules_sdk::exports::pb::rules::resolver::Target>, String> {
        tracing::debug!("hmmm, we're resolving a target");
        Ok(vec![])
    }
}

impl pb_rules_sdk::rules::RuleSet for StdRules {
    fn rule_set() -> Vec<(String, Box<dyn Rule>)> {
        vec![("http".to_string(), Box::new(HttpRule))]
    }
}

struct HttpRule;

impl pb_rules_sdk::rules::Rule for HttpRule {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "http".into()
    }

    fn execute(
        &self,
        _attrs: pb_rules_sdk::rules::Attributes,
        context: pb_rules_sdk::pb::rules::context::Ctx,
    ) -> BoxFuture<'static, Vec<pb_rules_sdk::pb::rules::types::Provider>> {
        Box::pin(async move {
            let request = pb_rules_sdk::pb::rules::http::Request {
                url: "https://jsonplaceholder.typicode.com/comments".into(),
                headers: vec![],
            };

            let mut body = Vec::new();
            let response = context.actions().http().get(&request).compat().await;
            let mut body_stream = ByteStreamWrapper::new(response.body());

            while let Some(val) = body_stream.next().await {
                tracing::info!("new chunk {}, total {}", val.len(), body.len());
                body.extend_from_slice(&val[..]);
            }

            let msg = String::from_utf8_lossy(&body[..]);
            tracing::info!(%msg, "got entire body");

            vec![]
        })
    }
}

pb_rules_sdk::export!(StdRules with_types_in pb_rules_sdk);
