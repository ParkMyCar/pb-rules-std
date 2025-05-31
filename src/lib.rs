use async_compression::tokio::bufread::XzDecoder;
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
                url: "https://github.com/llvm/llvm-project/releases/download/llvmorg-20.1.6/LLVM-20.1.6-Linux-X64.tar.xz".into(),
                headers: vec![],
            };

            let response = context.actions().http().get(&request).compat().await;
            tracing::info!(headers = ?response.headers(), "response headers");
            let body_stream = ByteStreamWrapper::new(response.body());

            let write_filesystem = context.actions().write_filesystem();

            let repository = write_filesystem
                .create_directory("testdir2")
                .compat()
                .await
                .expect("failed to create directory");
            let file = repository
                .create_file("testfile")
                .compat()
                .await
                .expect("failed to create test file");
            let mut wrote_bytes = 0;

            let byte_stream = body_stream.map(|v| Ok::<_, std::io::Error>(bytes::Bytes::from(v)));
            let async_reader = tokio_util::io::StreamReader::new(byte_stream);
            let reader = XzDecoder::new(async_reader);

            let mut stream = tokio_util::io::ReaderStream::new(reader);
            while let Some(val) = stream.next().await {
                let val = val.unwrap();
                wrote_bytes += val.len();
                file.append(&val[..])
                    .compat()
                    .await
                    .expect("failed to write content");
            }

            file.close()
                .compat()
                .await
                .expect("failed to write content");

            // Close the repository so it gets moved into place.
            repository.close().compat().await.unwrap();

            tracing::info!(%wrote_bytes, "got entire body");

            vec![]
        })
    }
}

pb_rules_sdk::export!(StdRules with_types_in pb_rules_sdk);
