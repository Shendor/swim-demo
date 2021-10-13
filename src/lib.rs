pub mod member_node;
pub mod network_router;
pub mod connection;
pub mod message;
mod tests;

use crate::network_router::{DefaultNodeRequestRouter, NodeRequestRouter};

pub fn run_network() -> Box<dyn NodeRequestRouter> {
    let mut router = DefaultNodeRequestRouter::new();
    router.start();
    Box::new(router)
}