use std::collections::{HashMap};
use std::ops::Not;
use std::sync::{Arc, Mutex};
use crate::member_node::{Message, DefaultMemberNode, ConnectionFactory, MemberNodeDetails};

pub trait NodeRequestRouter {
    fn start(&mut self);

    fn send_to(&mut self, from: u16, to: u16);

    fn shut_down(&self);
}

type Routes = HashMap<u16, Arc<Mutex<MemberNodeDetails>>>;

pub struct DefaultNodeRequestRouter {
    routes: Routes,
    // connection_factory: ConnectionFactory,
    connection_factory: Arc<Mutex<ConnectionFactory>>,
}

impl NodeRequestRouter for DefaultNodeRequestRouter {
    fn start(&mut self) {
        if self.routes.is_empty() {
            self.add_node(1)
        }
    }

    fn send_to(&mut self, from: u16, to: u16) {
        if self.routes.contains_key(&from).not() {
            self.add_node(from)
        }
        let from_node_details = self.routes.get(&from).unwrap().lock().unwrap().serialize();
        self.connection_factory.lock().unwrap().send_to(to, Message::Request(from_node_details, format!("hello from {}", from)))
    }

    fn shut_down(&self) {
        for node in self.routes.values() {
            self.connection_factory.lock().unwrap().send_to(node.lock().unwrap().id(), Message::Shutdown());
        }
    }
}

impl DefaultNodeRequestRouter {
    pub fn new() -> DefaultNodeRequestRouter {
        DefaultNodeRequestRouter {
            routes: Routes::new(),
            connection_factory: Arc::new(Mutex::new(ConnectionFactory::new())),
        }
    }

    fn add_node(&mut self, id: u16) {
        let node = DefaultMemberNode::new(id, Arc::clone(&self.connection_factory));
        // let node_ref = Arc::new(Mutex::new(node));
        self.routes.insert(id, Arc::clone(&node));

        // DefaultMemberNode::run_echo(node_ref);
        println!("Node {} has been added", id);
    }
}
