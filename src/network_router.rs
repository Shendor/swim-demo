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

type Routes<T> = HashMap<u16, Arc<Mutex<T>>>;

pub struct DefaultNodeRequestRouter<T> where T : MemberNode {
    routes: Routes<T>,
    connection_factory: Arc<Mutex<dyn ConnectionRegistry>>,
    node_factory: Box<dyn NodeFactory<T>>,
}

impl <T> NodeRequestRouter for DefaultNodeRequestRouter<T> where T : MemberNode {
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

impl <T> DefaultNodeRequestRouter<T> where T : MemberNode {
    pub fn new(node_factory: Box<dyn NodeFactory<T>>,
                  connection_registry: Arc<Mutex<dyn ConnectionRegistry + Send>>) -> Box<DefaultNodeRequestRouter<T>> {
        Box::new(DefaultNodeRequestRouter {
            routes: Routes::<T>::new(),
            connection_factory: connection_registry,
            node_factory,
        })
    }

    fn add_node(&mut self, host: u16) {
        self.routes.insert(host, self.node_factory.create(host, Arc::clone(&self.connection_factory)));

        println!("Node {} has been added", host);
    }
}

pub trait NodeFactory<T>
    where T: MemberNode {
    fn create(&self, host: u16, connection: Arc<Mutex<dyn ConnectionRegistry>>) -> Arc<Mutex<T>>;
}

pub struct DefaultNodeFactory;

impl NodeFactory<DefaultMemberNode> for DefaultNodeFactory {
    fn create(&self, host: u16, connection: Arc<Mutex<dyn ConnectionRegistry>>) -> Arc<Mutex<DefaultMemberNode>> {
        DefaultMemberNode::new(host, connection)
    }
}