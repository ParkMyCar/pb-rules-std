use std::io::Read;

use async_compression::tokio::bufread::ZstdDecoder;
use futures::{StreamExt, future::LocalBoxFuture};
use pb_rules_sdk::{
    futures::{ByteStreamWrapper, FutureCompat2},
    rules::Rule,
};
use tar::EntryType;

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
    ) -> LocalBoxFuture<'static, Vec<pb_rules_sdk::pb::rules::types::Provider>> {
        Box::pin(async move {
            let request = pb_rules_sdk::pb::rules::http::Request {
                url: "https://github.com/MaterializeInc/toolchains/releases/download/clang-19.1.6-2/darwin_aarch64.tar.zst".into(),
                headers: vec![],
            };

            let response = context.actions().http().get(&request).compat().await;
            tracing::info!(headers = ?response.headers(), "response headers");
            let body_stream = ByteStreamWrapper::new(response.body());

            // Create a temp file to download the archive into.
            let write_filesystem = context.actions().write_filesystem();
            let temp_file = write_filesystem
                .create_file("http-temp")
                .compat()
                .await
                .expect("creating temp file for download");

            // Decompress the data as we download it.
            let byte_stream = body_stream.map(|v| Ok::<_, std::io::Error>(bytes::Bytes::from(v)));
            let async_reader = tokio_util::io::StreamReader::new(byte_stream);
            let reader = ZstdDecoder::new(async_reader);
            let mut stream = tokio_util::io::ReaderStream::new(reader);

            // Write the archive into a temp file.
            while let Some(val) = stream.next().await {
                let val = val.unwrap();
                temp_file
                    .append(&val[..])
                    .compat()
                    .await
                    .expect("failed to write content");
            }

            // Create a repository to reconstruct the tar archive into.
            let repository = write_filesystem
                .create_directory("darwin_aarch64")
                .compat()
                .await
                .expect("failed to create dir");

            let mut archive = tar::Archive::new(temp_file.into_read().into_reader());
            let entries = archive.entries().expect("failed to read entries");

            for entry in entries {
                let mut entry = entry.expect("failed entry");
                let path = entry.path().expect("failed to read path");
                let path = path.to_str().expect("non UTF-8 path");

                match entry.header().entry_type() {
                    EntryType::Directory => {
                        tracing::trace!(?path, "creating directory");

                        repository
                            .create_directory(path)
                            .compat()
                            .await
                            .expect("failed to create child dir");
                    }
                    EntryType::Regular => {
                        tracing::trace!(?path, "creating file");

                        let file = repository
                            .create_file(path)
                            .compat()
                            .await
                            .expect("failed to create child file");

                        let mut buf = vec![0u8; 4096];
                        loop {
                            let bytes_read = entry.read(&mut buf[..]).expect("reading archive");

                            // We're done!
                            if bytes_read == 0 {
                                file.close().compat().await.expect("failed to close file");
                                break;
                            }

                            // Write into the repository.
                            file.append(&buf[..bytes_read])
                                .compat()
                                .await
                                .expect("failed to write file");
                        }
                    }
                    other => {
                        tracing::info!(?other, "skipping unsupported type");
                    }
                }

                tracing::info!(path = ?entry.path(), "got entry");
            }

            // Close the repository so it gets moved into place.
            repository
                .close()
                .compat()
                .await
                .expect("failed to close directory");

            vec![]
        })
    }
}

pb_rules_sdk::export!(StdRules with_types_in pb_rules_sdk);
