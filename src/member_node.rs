use std::{thread};
use std::borrow::BorrowMut;
use std::collections::{HashMap};
use std::hash::{Hash, Hasher};
use std::ops::Not;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rand;
use rand::{Rng, thread_rng};

pub enum Message {
    Request(MemberNodeDetails, String),
    Response(MemberNodeDetails, String),
    Ping(MemberNodeDetails, Option<MemberNodeDetails>),
    PingResponse(u16, Option<MemberNodeDetails>, bool),
    ProbeRequest(MemberNodeDetails, u16),
    ProbeResponse(u16, bool),
    Shutdown(),
}

pub trait MemberNode {
    fn send(&self, connection: Arc<Mutex<ConnectionFactory>>, message: Message);

    fn shut_down(&self, connection: Arc<Mutex<ConnectionFactory>>);
}

pub struct DefaultMemberNode {
    details: MemberNodeDetails,
    members: MemberNodesRegistry,
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, connection: Arc<Mutex<ConnectionFactory>>, message: Message) {
        connection.lock().unwrap().send_to(self.details.id, message);
    }

    fn shut_down(&self, connection: Arc<Mutex<ConnectionFactory>>) {
        connection.lock().unwrap().send_to(self.details.id, Message::Shutdown());
    }
}

impl DefaultMemberNode {
    pub fn new(id: u16, connection: Arc<Mutex<ConnectionFactory>>) -> MemberNodeDetails {
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let connection_ref = Arc::clone(&connection);
        connection.lock().unwrap().add_connection(id, sender);

        let node_details = MemberNodeDetails::new(id);
        let node = DefaultMemberNode {
            details: node_details,
            members: MemberNodesRegistry::new(),
        };
        let node_ref = Arc::new(Mutex::new(node));
        let node_ref_2 = Arc::clone(&node_ref);

        thread::spawn(move || {
            println!("Node {} started to listen requests", &id);
            loop {
                match receiver.recv().unwrap() {
                    Message::Request(from, data) => {
                        println!("Node {} received message: {}", id, data);

                        let mut node = node_ref.lock().unwrap();
                        node.members.add(from);

                        DefaultMemberNode::send_to(from.id, Message::Response(node.details, String::from("hi")), &connection);
                    }
                    Message::Response(from, data) => {
                        let mut node = node_ref.lock().unwrap();
                        node.members.add(from);

                        println!("Node {} received response from Node {}: {}", id, from.id, data)
                    }
                    Message::Ping(from, probing_node) => {
                        println!("Node {} received ping request from Node {}", &id, from.id);
                        DefaultMemberNode::send_to(from.id, Message::PingResponse(id, probing_node, false), &connection);
                    }
                    Message::PingResponse(from, probing_node, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        match probing_node {
                            Some(n) => {
                                DefaultMemberNode::send_to(n.id, Message::ProbeResponse(from, is_timed_out), &connection);
                            }
                            None => {
                                if is_timed_out {
                                    let mut node = node_ref.lock().unwrap();
                                    let details = node.details;
                                    node.members.set_node_state(from, MemberNodeState::Failed);
                                    for n in node.members.get_random_nodes(3).iter() {
                                        DefaultMemberNode::send_to(n.id, Message::ProbeRequest(details, from), &connection);
                                    }
                                } else {
                                    node_ref.lock().unwrap().members.set_node_state(from, MemberNodeState::Alive)
                                }
                            }
                        }
                    }
                    Message::ProbeRequest(from, timed_out_node) => {
                        let node = node_ref.lock().unwrap();
                        match node.members.get_by_id(timed_out_node) {
                            Some(n) => {
                                let details = node.details;
                                DefaultMemberNode::send_to(n.id, Message::Ping(details, Option::Some(from)), &connection);
                            }
                            _ => {}
                        }
                    }
                    Message::ProbeResponse(from, is_timed_out) => {
                        if is_timed_out.not() {
                            node_ref.lock().unwrap().members.set_node_state(from, MemberNodeState::Alive);
                        }
                    }
                    Message::Shutdown() => {
                        println!("Node {} received termination message", &id);
                        break;
                    }
                }
            }
        });

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3));
                let node = node_ref_2.lock().unwrap();
                match node.members.get_random_node() {
                    Some(n) => {
                        DefaultMemberNode::send_to(n.id, Message::Ping(node.details, Option::None), &connection_ref);
                    }
                    None => {}
                }
            }
        });
        node_details
    }

    fn send_to(id: u16, message: Message, connection_factory: &Arc<Mutex<ConnectionFactory>>) {
        connection_factory.lock().unwrap().send_to(id, message);
    }

    pub fn add_member_node(&mut self, member_node_details: MemberNodeDetails) {
        self.members.add(member_node_details);
    }

    pub fn details(&self) -> MemberNodeDetails { self.details }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemberNodeState {
    Alive,
    Suspected,
    Failed,
}

#[derive(Clone, Copy, Eq)]
pub struct MemberNodeDetails {
    id: u16,
    state: MemberNodeState,
}

impl Hash for MemberNodeDetails {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl PartialEq for MemberNodeDetails {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
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
    members: HashMap<u16, MemberNodeDetails>,
}

impl MemberNodesRegistry {
    pub fn new() -> Self {
        MemberNodesRegistry {
            members: HashMap::new()
        }
    }

    pub fn add(&mut self, member_node_details: MemberNodeDetails) {
        self.members.insert(member_node_details.id, member_node_details);
    }

    pub fn set_node_state(&mut self, id: u16, state: MemberNodeState) {
        match self.members.get_mut(&id) {
            Some(n) => { n.borrow_mut().change_state(state) }
            _ => {}
        }
    }

    pub fn get_by_id(&self, id: u16) -> Option<&MemberNodeDetails> {
        self.members.get(&id)
    }

    pub fn get_random_node(&self) -> Option<&MemberNodeDetails> {
        if self.is_empty() {
            None
        } else {
            let members: Vec<&MemberNodeDetails> = self.members.values()
                .filter(|m| m.state == MemberNodeState::Alive)
                .collect();

            let random_index = thread_rng().gen_range(0..members.len());
            let random_node = &members[random_index];
            Some(random_node)
        }
    }

    pub fn get_random_nodes(&self, number: usize) -> Vec<&MemberNodeDetails> {
        use rand::prelude::*;
        let mut members: Vec<&MemberNodeDetails> = self.members.values()
            .filter(|m| m.state == MemberNodeState::Alive)
            .collect();

        members.shuffle(&mut rand::thread_rng());
        members.iter().take(number).cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

pub struct ConnectionFactory {
    connection: HashMap<u16, Sender<Message>>,
}

impl ConnectionFactory {
    pub fn new() -> ConnectionFactory {
        ConnectionFactory {
            connection: HashMap::new()
        }
    }

    pub fn get_connection_for(&self, id: u16) -> Option<&Sender<Message>> {
        self.connection.get(&id)
    }

    pub fn send_to(&self, id: u16, message: Message) {
        match self.connection.get(&id) {
            Some(c) => { c.send(message); }
            _ => {}
        }
    }

    pub fn add_connection(&mut self, id: u16, connection: Sender<Message>) {
        self.connection.insert(id, connection);
    }

    pub fn remove_connection(&mut self, id: u16) {
        self.connection.remove(&id);
    }
}