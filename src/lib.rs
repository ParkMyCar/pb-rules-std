//! A standard library of rules for the `pb` build system.

use futures::{StreamExt, future::LocalBoxFuture};
use headers::Header;
use pb_rules_sdk::{
    futures::{ByteStreamWrapper, FutureCompat2},
    pb::rules::types::{Attribute, AttributeKind, AttributeSpec},
    rules::Rule,
};

mod util;

struct StdRules;

impl pb_rules_sdk::resolver::Resolver for StdRules {
    type Iterator = StdRulesTargetDiffIter;

    fn new() -> Self {
        StdRules
    }

    fn additional_interest_glob() -> Option<String> {
        tracing::info!("logging from additional interests glob");
        Some("pb.toml".into())
    }

    fn process_update(
        &self,
        _update: pb_rules_sdk::exports::pb::rules::target_resolver::ManifestUpdate,
    ) {
        todo!()
    }

    fn target_diffs(&self) -> Self::Iterator {
        todo!()
    }
}

struct StdRulesTargetDiffIter;

impl pb_rules_sdk::resolver::TargetDiffIterator for StdRulesTargetDiffIter {
    fn next(&self) -> Option<pb_rules_sdk::exports::pb::rules::target_resolver::ResolvedTarget> {
        None
    }
}

impl pb_rules_sdk::rules::RuleSet for StdRules {
    fn rule_set() -> Vec<(String, Box<dyn Rule>)> {
        let rules: [Box<dyn Rule>; 1] = [Box::new(HttpRule)];

        rules
            .into_iter()
            .map(|rule| (rule.name().to_string(), rule))
            .collect()
    }
}

struct HttpRule;

impl pb_rules_sdk::rules::Rule for HttpRule {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "http-repository".into()
    }

    fn spec(&self) -> pb_rules_sdk::exports::pb::rules::rules::RuleSpec {
        pb_rules_sdk::exports::pb::rules::rules::RuleSpec {
            attributes: vec![
                AttributeSpec {
                    name: "name".to_string(),
                    kind: AttributeKind::Text,
                    required: true,
                },
                AttributeSpec {
                    name: "url".to_string(),
                    kind: AttributeKind::Text,
                    required: true,
                },
                AttributeSpec {
                    name: "integrity".to_string(),
                    kind: AttributeKind::Text,
                    required: false,
                },
            ],
            repository: true,
        }
    }

    fn execute(
        &self,
        mut attrs: pb_rules_sdk::rules::Attributes,
        context: pb_rules_sdk::pb::rules::context::Ctx,
    ) -> LocalBoxFuture<'static, Vec<pb_rules_sdk::pb::rules::types::Provider>> {
        tracing::info!(?attrs, "http-repository");

        let Some(Attribute::Text(name)) = attrs.inner.remove("name") else {
            panic!("name is a required attribute");
        };
        let Some(Attribute::Text(url)) = attrs.inner.remove("url") else {
            panic!("url is a required attribute");
        };

        Box::pin(async move {
            let request = pb_rules_sdk::pb::rules::http::Request {
                url,
                headers: vec![],
            };

            // Send our request.
            let response = context.actions().http().get(&request).compat().await;
            let headers = response.headers();
            tracing::info!(?headers, "response headers");

            let body_stream = ByteStreamWrapper::new(response.body())
                .map(|v| Ok::<_, std::io::Error>(bytes::Bytes::from(v)));
            let content_encoding = headers
                .iter()
                .find(|(name, _value)| name == headers::ContentEncoding::name().as_str())
                .map(|(_name, value)| value.as_str());
            let content_disposition = headers
                .iter()
                .find(|(name, _value)| name == headers::ContentDisposition::name().as_str())
                .map(|(_name, value)| value.as_str());

            // Wrap our stream to decompress it, if necessary.
            let mut body_stream = crate::util::decompress_stream(
                body_stream,
                content_encoding,
                content_disposition,
                None,
            )
            .expect("failed to decompress stream");

            // Create a temp file to download the archive into.
            let write_filesystem = context.actions().write_filesystem();
            let temp_file = write_filesystem
                .create_file("http-temp")
                .compat()
                .await
                .expect("creating temp file for download");

            // Write the archive into a temp file.
            while let Some(val) = body_stream.next().await {
                let val = val.expect("error in stream!");
                temp_file
                    .append(&val[..])
                    .compat()
                    .await
                    .expect("failed to write content");
            }

            // Create a repository to reconstruct the tar archive into.
            let repository = write_filesystem
                .create_directory(&name)
                .compat()
                .await
                .expect("failed to create dir");
            let repository =
                crate::util::reconstruct_tar(repository, temp_file.into_read().into_reader())
                    .await
                    .expect("failed to reconstruct tar");

            // Close the repository, moving it into place.
            repository
                .close()
                .compat()
                .await
                .expect("failed to close the repository");

            vec![]
        })
    }
}

pb_rules_sdk::export!(StdRules with_types_in pb_rules_sdk);
