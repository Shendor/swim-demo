use std::cell::RefCell;
use std::collections::{HashMap};
use std::ops::Not;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use crate::member_node::{MemberNode, Message, DefaultMemberNode};

pub trait NodeRequestRouter {
    fn start(&mut self);

    fn send_to(&mut self, from: u16, to: u16);

    fn shut_down(&self);
}

type Routes = HashMap<u16, Arc<Mutex<DefaultMemberNode>>>;

pub struct DefaultNodeRequestRouter {
    routes: Routes,
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
        let node = self.routes.get(&to).unwrap();
        let from_node = self.routes.get(&from).unwrap();
        // let from_node_arc = Arc::new(Mutex::new(DefaultMemberNode::new(12)));
        // node.unwrap().borrow().send(Message::DATA(String::from("hello from " + from.to_string())));
        node.lock().unwrap().send(Message::DATA(format!("hello from {}", from), Arc::clone(from_node)));
        // node.lock().unwrap().send(Message::JOIN(Arc::clone(from_node)));
    }

    fn shut_down(&self) {
        for node in self.routes.values() {
            node.lock().unwrap().shut_down()
        }
    }
}

impl DefaultNodeRequestRouter {
    pub fn new() -> DefaultNodeRequestRouter {
        DefaultNodeRequestRouter {
            routes: Routes::new(),
        }
    }

    fn add_node(&mut self, id: u16) {
        let node = DefaultMemberNode::new(id);
        self.routes.insert(id, Arc::new(Mutex::new(node)));
        println!("Node {} has been added", id);
    }
}
