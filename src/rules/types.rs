use std::collections::BTreeMap;
use std::future::Future;

pub struct Provider;

pub struct Context;

// pub trait Rule {
//     fn name() -> String;

//     fn run(
//         attrs: BTreeMap<String, Attribute>,
//         context: Context,
//     ) -> Box<dyn Future<Output = Vec<Provider>> + Send + 'static>;
// }
