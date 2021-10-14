pub mod member_node;
pub mod network_router;
pub mod connection;
pub mod message;
mod tests;

use std::sync::{Arc, Mutex};
use crate::connection::swim_node::ConnectionFactory;
use crate::network_router::{DefaultNodeRequestRouter, NodeRequestRouter, DefaultNodeFactory};

pub fn run_network() -> Box<dyn NodeRequestRouter> {
    let node_factory = DefaultNodeFactory {};
    let mut router = DefaultNodeRequestRouter::new(Box::<DefaultNodeFactory>::new(node_factory), Arc::new(Mutex::new(ConnectionFactory::new())));
    router.start();
    router
}