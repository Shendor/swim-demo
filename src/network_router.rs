use std::collections::{HashMap};
use std::ops::Not;
use std::sync::{Arc, Mutex};
use crate::connection::swim_node::{ConnectionRegistry};
use crate::member_node::swim_node::{DefaultMemberNode, MemberNode};
use crate::message::swim_node::Message;

pub trait NodeRequestRouter {
    fn start(&mut self);

    fn send(&mut self, from: u16, to: u16);

    fn shut_down(&self);
}

type Routes = HashMap<u16, Arc<Mutex<dyn MemberNode>>>;

pub struct DefaultNodeRequestRouter {
    routes: Routes,
    connection_factory: Arc<Mutex<dyn ConnectionRegistry>>,
    node_factory: Box<dyn NodeFactory<DefaultMemberNode>>
}

impl NodeRequestRouter for DefaultNodeRequestRouter {
    fn start(&mut self) {
        if self.routes.is_empty() {
            self.add_node(1)
        }
    }

    fn send(&mut self, from: u16, to: u16) {
        if self.routes.contains_key(&from).not() {
            self.add_node(from)
        }
        let from_node_details = self.routes.get(&from).unwrap().lock().unwrap().serialize_host_details();
        self.connection_factory.lock().unwrap().send_to(to, Message::Request(from_node_details, format!("hello from {}", from)))
    }

    fn shut_down(&self) {
        for node in self.routes.values() {
            self.connection_factory.lock().unwrap().send_to(node.lock().unwrap().host(), Message::Shutdown());
        }
    }
}

impl DefaultNodeRequestRouter {
    pub fn new(node_factory : Box<dyn NodeFactory<DefaultMemberNode>>,
               connection_registry : Arc<Mutex<dyn ConnectionRegistry>>) -> Box<DefaultNodeRequestRouter> {
        Box::new(DefaultNodeRequestRouter {
            routes: Routes::new(),
            connection_factory:  connection_registry,
            node_factory
        })
    }
    fn add_node(&mut self, id: u16) {
        self.routes.insert(id,  self.node_factory.create(id,  Arc::clone(&self.connection_factory)));

        println!("Node {} has been added", id);
    }
}

pub trait NodeFactory<T>
    where T: MemberNode {
    fn create(&self, id: u16, connection: Arc<Mutex<dyn ConnectionRegistry>>) -> Arc<Mutex<T>>;
}

pub struct DefaultNodeFactory;

impl NodeFactory<DefaultMemberNode> for DefaultNodeFactory {
    fn create(&self, id: u16, connection: Arc<Mutex<dyn ConnectionRegistry>>) -> Arc<Mutex<DefaultMemberNode>> {
        DefaultMemberNode::new(id, connection)
    }
}