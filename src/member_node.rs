use std::{thread, vec};
use std::ops::Not;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rand;
use rand::{Rng, thread_rng};

pub type MemberNodesRegistryDetails = (MemberNodeDetails, Arc<Mutex<Sender<Message>>>);

pub enum Message {
    Request(String, MemberNodesRegistryDetails),
    Response(MemberNodesRegistryDetails, String),
    Ping(MemberNodesRegistryDetails, Option<MemberNodesRegistryDetails>),
    PingResponse(u16, Option<MemberNodesRegistryDetails>, bool),
    ProbeRequest(MemberNodesRegistryDetails, u16),
    ProbeResponse(u16, bool),
    Shutdown(),
}

pub trait MemberNode {
    fn send(&self, message: Message);

    fn shut_down(&self);
}

pub struct DefaultMemberNode {
    details: MemberNodeDetails,
    members: MemberNodesRegistry,
    sender: Arc<Mutex<Sender<Message>>>,
    is_terminated: bool,
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, message: Message) {
        self.sender.lock().unwrap().send(message).unwrap();
    }

    fn shut_down(&self) {
        self.send(Message::Shutdown());
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
                members: MemberNodesRegistry::new(),
                sender: sender_ref,
                is_terminated: false,
            }));
        let node_ref = Arc::clone(&node);
        let node_ref_2 = Arc::clone(&node_ref);

        thread::spawn(move || {
            println!("Node {} started to listen requests", &id);
            loop {
                match receiver.recv().unwrap() {
                    Message::Request(d, from) => {
                        println!("Node {} received message: {}", id, d);

                        let mut node = node_ref.lock().unwrap();
                        let details = node.details;
                        let connection = Arc::clone(&node.sender);
                        node.members.add((from.0, Arc::clone(&from.1)));

                        from.1.lock().unwrap().send(Message::Response((details, connection), String::from("hi")));
                    }
                    Message::Response(from, data) => {
                        let mut node = node_ref.lock().unwrap();
                        let from_details = from.0;
                        node.members.add(from);
                        println!("Node {} received response from Node {}: {}", id, from_details.id, data)
                    }
                    Message::Ping(from, probing_node) => {
                        println!("Node {} received ping request from Node {}", &id, from.0.id);
                        from.1.lock().unwrap().send(Message::PingResponse(id, probing_node, false));
                    }
                    Message::PingResponse(from, probing_node, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        match probing_node {
                            Some(n) => {
                                n.1.lock().unwrap().send(Message::ProbeResponse(from, is_timed_out));
                            }
                            None => {
                                if is_timed_out {
                                    let mut node = node_ref.lock().unwrap();
                                    let details = node.details;
                                    node.members.set_node_state(from, MemberNodeState::Failed);
                                    for n in node.members.get_random_nodes(3).iter() {
                                        n.1.lock().unwrap().send(Message::ProbeRequest((details, Arc::clone(&node.sender)), from));
                                    }
                                } else {
                                    node_ref.lock().unwrap().members.set_node_state(from, MemberNodeState::Alive)
                                }
                            }
                        }
                    }
                    Message::ProbeRequest(from, timed_out_node) => {
                        let node = node_ref.lock().unwrap();
                        let details = node.details;
                        let connection = Arc::clone(&node.sender);

                        match node.members.get_by_id(timed_out_node) {
                            Some(n) => {
                                n.1.lock().unwrap().send(Message::Ping((details, Arc::clone(&connection)), Option::Some(from)));
                            }
                            _ => {}
                        }
                    }
                    Message::ProbeResponse(from, is_timed_out) => {
                        let mut node = node_ref.lock().unwrap();
                        if is_timed_out.not() {
                            node.members.set_node_state(from, MemberNodeState::Alive);
                        }
                    }
                    Message::Shutdown() => {
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
                        n.1.lock().unwrap().send(Message::Ping((node.details, Arc::clone(&node.sender)), Option::None));
                    }
                    None => {}
                }
                if node.is_terminated { break; }
            }
        });
        node
    }

    pub fn add_member_node(&mut self, member_node_details: MemberNodesRegistryDetails) {
        self.members.add(member_node_details);
    }

    pub fn details(&self) -> MemberNodeDetails { self.details }

    pub fn connection(&self) -> Arc<Mutex<Sender<Message>>> { Arc::clone(&self.sender) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemberNodeState {
    Alive,
    Suspected,
    Failed,
}

#[derive(Clone, Copy)]
pub struct MemberNodeDetails {
    id: u16,
    state: MemberNodeState,
}

impl MemberNodeDetails {
    pub fn new(id: u16) -> Self {
        MemberNodeDetails {
            id,
            state: MemberNodeState::Alive,
        }
    }

    pub fn id(&self) -> u16 { self.id }

    pub fn name(&self) -> String { format!("node-{}", self.id) }

    pub fn state(&self) -> &MemberNodeState { &self.state }

    pub fn change_state(&mut self, state: MemberNodeState) { self.state = state }
}

pub struct MemberNodesRegistry {
    members: Vec<MemberNodesRegistryDetails>,
}

impl MemberNodesRegistry {
    pub fn new() -> Self {
        MemberNodesRegistry {
            members: vec!()
        }
    }

    pub fn add(&mut self, member_node_details: MemberNodesRegistryDetails) {
        self.members.push(member_node_details);
    }

    pub fn set_node_state(&mut self, id: u16, state: MemberNodeState) {
        match self.members.iter_mut().find(|ref i| { i.0.id == id }) {
            Some(n) => { n.0.change_state(state) }
            _ => {}
        }
    }

    pub fn get_by_id(&self, id: u16) -> Option<&MemberNodesRegistryDetails> {
        self.members.iter().find(|ref i| { i.0.id == id })
    }

    pub fn get_random_node(&self) -> Option<&MemberNodesRegistryDetails> {
        if self.is_empty() {
            None
        } else {
            let random_index = thread_rng().gen_range(0..self.members.len());
            let random_node = &self.members[random_index];
            Some(random_node)
        }
    }

    pub fn get_random_nodes(&self, number: usize) -> Vec<&MemberNodesRegistryDetails> {
        use rand::prelude::*;
        let mut possible_members: Vec<&MemberNodesRegistryDetails> = self.members
            .iter()
            .filter(|m| m.0.state == MemberNodeState::Alive)
            .collect();
        let mut rng = rand::thread_rng();

        possible_members.shuffle(&mut rng);

        possible_members.iter().take(number).cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}