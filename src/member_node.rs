use std::ops::{Not, Range};
use std::sync::mpsc::{Receiver, Sender};
use rand;
use rand::{Rng, thread_rng};
use std::{thread, vec};
use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

pub enum Message {
    DATA(String, (MemberNodeDetails, Arc<Mutex<Sender<Message>>>)),
    RESPONSE((MemberNodeDetails, Arc<Mutex<Sender<Message>>>), String),
    PING((MemberNodeDetails, Arc<Mutex<Sender<Message>>>)),
    PING_RESPONSE(u16, bool),
    SHUTDOWN(),
}

pub trait MemberNode {
    fn send(&self, message: Message);

    fn shut_down(&self);
}

pub struct DefaultMemberNode {
    details: MemberNodeDetails,
    members: MemberNodesRegistry2,
    sender: Arc<Mutex<Sender<Message>>>,
    is_terminated: bool,
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, message: Message) {
        self.sender.lock().unwrap().send(message).unwrap();
    }

    fn shut_down(&self) {
        self.send(Message::SHUTDOWN());
        println!("Node {} is shutting down", self.details.id);
    }
}

impl DefaultMemberNode {
    pub fn new(id: u16) -> Arc<Mutex<DefaultMemberNode>> {
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let sender_ref = Arc::new(Mutex::new(sender));
        let node = Arc::new(Mutex::new(
            DefaultMemberNode {
                details: MemberNodeDetails::new(id),
                members: MemberNodesRegistry2::new(),
                sender: sender_ref,
                is_terminated: false,
            }));
        let node_ref = Arc::clone(&node);
        let node_ref_2 = Arc::clone(&node_ref);

        thread::spawn(move || {
            println!("Node {} started to listen requests", &id);
            loop {
                match receiver.recv().unwrap() {
                    Message::DATA(d, from) => {
                        println!("Node {} received message: {}", id, d);

                        let mut node = node_ref.lock().unwrap();
                        let details = node.details;
                        let connection = Arc::clone(&node.sender);
                        node.members.add((from.0, Arc::clone(&from.1)));

                        from.1.lock().unwrap().send(Message::RESPONSE((details, connection), String::from("hi")));
                    }
                    Message::RESPONSE(from, data) => {
                        let mut node = node_ref.lock().unwrap();
                        let from_details = from.0;
                        node.members.add(from);
                        println!("Node {} received response from Node {}: {}", id, from_details.id, data)
                    }
                    Message::PING(from) => {
                        // let from_node = from.lock().unwrap();
                        println!("Node {} received ping request from Node {}", &id, from.0.id);
                        from.1.lock().unwrap().send(Message::PING_RESPONSE(id, false));
                    }
                    Message::PING_RESPONSE(from, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        // node_ref.lock().unwrap().members.lock().unwrap().set_node_state(from, if is_timed_out { MemberNodeState::SUSPECTED } else { MemberNodeState::ALIVE })
                    }
                    Message::SHUTDOWN() => {
                        println!("Node {} received termination message", &id);
                        let mut node = node_ref.lock().unwrap();
                        node.is_terminated = true;
                        break;
                    }
                }
            }
        });

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));
                let node = node_ref_2.lock().unwrap();
                match node.members.get_random_node() {
                    Some(n) => {
                        n.1.lock().unwrap().send(Message::PING((node.details, Arc::clone(&node.sender))));
                    }
                    None => {
                    }
                }
                if node.is_terminated { break; }
            }
        });
        node
    }

    pub fn add_member_node(&mut self, node: (MemberNodeDetails, Arc<Mutex<Sender<Message>>>)) {
        self.members.add(node);
    }

    pub fn details(&self) -> MemberNodeDetails { self.details }

    pub fn connection(&self) -> Arc<Mutex<Sender<Message>>> { Arc::clone(&self.sender) }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Copy)]
pub enum MemberNodeState {
    ALIVE,
    SUSPECTED,
    FAILED,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Copy)]
pub struct MemberNodeDetails {
    id: u16,
    state: MemberNodeState,
}

impl MemberNodeDetails {
    pub fn new(id: u16) -> Self {
        MemberNodeDetails {
            id,
            state: MemberNodeState::ALIVE,
        }
    }

    pub fn id(&self) -> u16 { self.id }

    pub fn name(&self) -> String { format!("node-{}", self.id) }

    pub fn state(&self) -> &MemberNodeState { &self.state }

    pub fn change_state(&mut self, state: MemberNodeState) { self.state = state }
}

pub struct MemberNodesRegistry2 {
    members: Vec<(MemberNodeDetails, Arc<Mutex<Sender<Message>>>)>,
}

impl MemberNodesRegistry2 {
    pub fn new() -> Self {
        MemberNodesRegistry2 {
            members: vec!()
        }
    }

    pub fn add(&mut self, node: (MemberNodeDetails, Arc<Mutex<Sender<Message>>>)) {
        self.members.push(node);
    }

    pub fn set_node_state(&mut self, id: u16, state: MemberNodeState) {
        match self.members.iter_mut().find(|ref i| { i.0.id == id }) {
            Some(mut n) => { n.0.change_state(state) }
            _ => {}
        }
    }

    pub fn get_random_node(&self) -> Option<&(MemberNodeDetails, Arc<Mutex<Sender<Message>>>)> {
        if self.is_empty() {
            None
        } else {
            let random_index = thread_rng().gen_range(0..self.members.len());
            let random_node = &self.members[random_index];
            Some(random_node)
        }
    }

    fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

pub struct MemberNodesRegistry {
    members: Vec<Arc<Mutex<DefaultMemberNode>>>,
}

impl MemberNodesRegistry {
    pub fn new() -> Self {
        MemberNodesRegistry {
            members: vec!()
        }
    }

    pub fn add(&mut self, node: Arc<Mutex<DefaultMemberNode>>) {
        self.members.push(node);
    }

    pub fn set_node_state(&mut self, id: u16, state: MemberNodeState) {
        match self.members.iter().find(|ref i| { i.lock().unwrap().details.id == id }) {
            Some(n) => { n.lock().unwrap().details.state = state }
            _ => {}
        }
    }

    pub fn get_random_node(&self) -> Option<Arc<Mutex<DefaultMemberNode>>> {
        if self.is_empty() {
            None
        } else {
            let random_index = thread_rng().gen_range(0..self.members.len());
            let random_node = &self.members[random_index];
            Some(Arc::clone(random_node))
        }
    }

    fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}