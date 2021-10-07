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
    RESPONSE(Arc<Mutex<DefaultMemberNode>>, String),
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
    is_terminated: bool,
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
    pub fn new(id: u16) -> Arc<Mutex<DefaultMemberNode>> {
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();

        let node = Arc::new(Mutex::new(
            DefaultMemberNode {
                details: MemberNodeDetails::new(id),
                members: Arc::new(Mutex::new(MemberNodesRegistry::new())),
                sender,
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

                        let node = node_ref.lock().unwrap();
                        node.members.lock().unwrap().add(Arc::clone(&from));

                        let from_node = from.lock().unwrap();
                        // from_node.members.lock().unwrap().add(Arc::clone(&node_ref));

                        from_node.send(Message::RESPONSE(Arc::clone(&node_ref), String::from("hi")));
                    }
                    Message::RESPONSE(from, data) => {
                        let node = node_ref.lock().unwrap();
                        node.members.lock().unwrap().add(Arc::clone(&from));
                        println!("Node {} received response from Node {}: {}", id, from.lock().unwrap().details.id, data)
                    }
                    Message::PING(from) => {
                        let from_node = from.lock().unwrap();
                        println!("Node {} received ping request from Node {}", &id, from_node.details.id);
                        from_node.send(Message::PING_RESPONSE(id, false))
                    }
                    Message::PING_RESPONSE(from, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        node_ref.lock().unwrap().members.lock().unwrap().set_node_state(from, if is_timed_out { MemberNodeState::SUSPECTED } else { MemberNodeState::ALIVE })
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

        if id == 1 {
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(1));
                    match node_ref_2.lock().unwrap().members.lock().unwrap().get_random_node() {
                        Some(n) => {
                            println!("got random node");

                            // println!("Random node {} received", guard.details.id);
                            n.lock().unwrap().send(Message::PING(Arc::clone(&node_ref_2)));
                        }
                        None => {
                            println!("no members found");
                        }
                    }

                    // if node.is_terminated { break; }
                }
            });
        }
        node
    }

    pub fn add_member_node(&mut self, node: Arc<Mutex<DefaultMemberNode>>) {
        self.members.lock().unwrap().add(node);
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