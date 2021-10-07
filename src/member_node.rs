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
    DATA(String, Arc<Mutex<DefaultMemberNode>>),
    RESPONSE(u16, String),
    JOIN(Arc<Mutex<DefaultMemberNode>>),
    PING(Arc<Mutex<DefaultMemberNode>>),
    PING_RESPONSE(u16, bool),
    SHUTDOWN(),
}

pub trait MemberNode {
    fn send(&self, message: Message);

    fn shut_down(&self);
}

pub struct DefaultMemberNode {
    details: MemberNodeDetails,
    members: Arc<Mutex<MemberNodesRegistry>>,
    sender: Sender<Message>,
    is_terminated: bool
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, message: Message) {
        self.sender.send(message).unwrap();
    }

    fn shut_down(&self) {
        self.send(Message::SHUTDOWN());
        println!("Node {} is shutting down", self.details.id);
    }
}

impl DefaultMemberNode {
    pub fn new(id: u16) -> Self {
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let members = Arc::new(Mutex::new(MemberNodesRegistry::new()));
        let members_ref = Arc::clone(&members);

        let is_terminated = false;
        let mut is_terminated_ref = Arc::new(Mutex::new(is_terminated));

        thread::spawn(move || {
            println!("Node {} started to listen requests", &id);
            while is_terminated_ref.lock().unwrap().not() {
                match receiver.recv().unwrap() {
                    Message::DATA(d, from) => {
                        println!("Node {} received message: {}", id, d);
                        members.lock().unwrap().add(Arc::clone(&from));

                        let from_node = from.lock().unwrap();
                        from_node.send(Message::RESPONSE(id, String::from("hi")));
                    }
                    Message::RESPONSE(from_id, data) => { println!("Node {} received response from Node {}: {}", id, from_id, data) }
                    Message::JOIN(n) => {
                        members.lock().unwrap().add(Arc::clone(&n))
                    }
                    Message::PING(from) => {
                        let from_node = from.lock().unwrap();
                        println!("Node {} received ping request from Node {}", &id, from_node.details.id);
                        from_node.send(Message::PING_RESPONSE(id, false))
                    }
                    Message::PING_RESPONSE(from, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        let mut node = members.lock().unwrap();
                        node.set_node_state(from, if is_timed_out { MemberNodeState::SUSPECTED } else { MemberNodeState::ALIVE })
                    }
                    Message::SHUTDOWN() => {
                        println!("Node {} received termination message", &id);
                        *is_terminated_ref.lock().unwrap() = true;
                    }
                }
            }
        });

        DefaultMemberNode {
            details: MemberNodeDetails::new(id),
            members: members_ref,
            sender,
            is_terminated,
        }
    }

    pub fn add_member_node(&mut self, node: Arc<Mutex<DefaultMemberNode>>) {
        self.members.lock().unwrap().add(node);
    }

    pub fn run_echo(this: Arc<Mutex<Self>>) {
        thread::spawn(move || {
            while this.lock().unwrap().is_terminated.not() {
                thread::sleep(Duration::from_secs(3));
                match this.lock().unwrap().members.lock().unwrap().get_random_node() {
                    Some(n) => {
                        n.lock().unwrap().send(Message::PING(Arc::clone(&this)));
                    }
                    None => {}
                }
            }
        });
    }
}

pub enum MemberNodeState {
    ALIVE,
    SUSPECTED,
    FAILED,
}

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

    pub fn get_random_node(&self) -> Option<&Arc<Mutex<DefaultMemberNode>>> {
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