wit_bindgen::generate!({
    world: "target-resolver",
    path: "pb-wit/wit"
});

struct HttpResolver;

impl exports::pb::rules::resolver::Guest for HttpResolver {
    fn additional_interest_glob() -> Option<_rt::String> {
        let location = pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        pb::rules::logging::event(
            pb::rules::logging::Level::Info,
            "additional interest",
            &location,
            &[],
        );
        Some("pb.toml".into())
    }

    fn resolve_target(
        _file: exports::pb::rules::resolver::File,
    ) -> Result<_rt::Vec<exports::pb::rules::resolver::Target>, _rt::String> {
        let location = pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        pb::rules::logging::event(
            pb::rules::logging::Level::Info,
            "resolving target",
            &location,
            &[],
        );

        Ok(vec![])
    }
}

export!(HttpResolver);
