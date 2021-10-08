pub mod member_node;
pub mod network_router;

use crate::network_router::{DefaultNodeRequestRouter, NodeRequestRouter};

/*
1. Run nodes in threads
2. Each node has Channel object and all of them waiting for a request using channel.read()
3. When request received for a node, it registers the sender if not exist in the member list
4. On startup, only 1 node is created and doing work from 2.
5. Every n-sec all nodes probing random nodes as per protocol. Probing message includes list of members
6. If node wants to join network, it sends a message to node id using Channel
*/
pub fn run_network() -> Box<dyn NodeRequestRouter> {
    let mut router = DefaultNodeRequestRouter::new();
    router.start();
    Box::new(router)
}