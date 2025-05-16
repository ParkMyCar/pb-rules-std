use std::future::Future;

use crate::StdRules;

pub mod http;
pub mod types;

impl super::exports::pb::rules::rules::Guest for StdRules {
    type Rule = StdRule;
    type Ctx = Context;
    type Actions = Actions;
    type RuleFuture = StdRuleFuture;

    fn rule_set() -> super::_rt::Vec<(super::_rt::String, super::exports::pb::rules::rules::Rule)> {
        let location = super::pb::rules::logging::Location {
            file_path: None,
            line: None,
        };
        super::pb::rules::logging::event(
            super::pb::rules::logging::Level::Info,
            "rule set",
            &location,
            &[],
        );

        vec![(
            "http".into(),
            super::exports::pb::rules::rules::Rule::new(StdRule::default()),
        )]
    }
}

#[derive(Default)]
pub struct StdRuleFuture;

impl super::exports::pb::rules::rules::GuestRuleFuture for StdRuleFuture {
    fn poll(
        &self,
        waker: &super::exports::pb::rules::rules::Waker,
    ) -> super::exports::pb::rules::rules::RulePoll {
        super::exports::pb::rules::rules::RulePoll::Pending
    }
}

#[derive(Default)]
pub struct StdRule;

impl super::exports::pb::rules::rules::GuestRule for StdRule {
    fn name() -> super::_rt::String {
        "TODO".into()
    }

    fn run(
        &self,
        attrs: super::_rt::Vec<(
            super::_rt::String,
            super::exports::pb::rules::rules::Attribute,
        )>,
        context: super::exports::pb::rules::rules::Ctx,
    ) -> super::exports::pb::rules::rules::RuleFuture {
        todo!()
    }
}

pub struct Context;

impl super::exports::pb::rules::rules::GuestCtx for Context {
    fn actions(&self) -> super::exports::pb::rules::rules::Actions {
        todo!()
    }
}

#[derive(Default)]
pub struct Actions;

impl super::exports::pb::rules::rules::GuestActions for Actions {
    fn run_wasm(&self, name: super::_rt::String) -> () {}
}
