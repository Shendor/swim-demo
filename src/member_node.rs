use std::{thread};
use std::collections::{HashMap};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Not};
use std::sync::{Arc, mpsc, Mutex, MutexGuard};
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
    // details: Arc<Mutex<MemberNodeDetails>>
    details: MemberNodeDetails,
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, connection: Arc<Mutex<ConnectionFactory>>, message: Message) {
        // connection.lock().unwrap().send_to(self.details.lock().unwrap().id, message);
    }

    fn shut_down(&self, connection: Arc<Mutex<ConnectionFactory>>) {
        connection.lock().unwrap().send_to(self.details.id, Message::Shutdown());
    }
}

impl DefaultMemberNode {
    pub fn new(id: u16, connection: Arc<Mutex<ConnectionFactory>>) -> Arc<Mutex<MemberNodeDetails>> {
        let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let connection_ref = Arc::clone(&connection);
        connection.lock().unwrap().add_connection(id, sender);

        let node_details = Arc::new(Mutex::new(MemberNodeDetails::new(id)));
        let node = DefaultMemberNode {
            // details: Arc::clone(&node_details)
            details: MemberNodeDetails::new(id)
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
                        node.add_member_node(from.id);

                        DefaultMemberNode::send_to(from.id, Message::Response(node.serialize(), String::from("hi")), &connection);
                    }
                    Message::Response(from, data) => {
                        let mut node = node_ref.lock().unwrap();

                        // node.add_member_nodes(&from_node.members);
                        node.add_member_node(from.id);

                        println!("Node {} received response from Node {}: {}", id, from.id, data)
                    }
                    Message::Ping(from, probing_node) => {
                        let mut node = node_ref.lock().unwrap();
                        node.add_member_nodes(&from.members);

                        println!("Node {} received ping request from Node {}, with members: {}", &id, from.id, node.details.members);

                        DefaultMemberNode::send_to(from.id, Message::PingResponse(id, probing_node, false), &connection);
                    }
                    Message::PingResponse(from, probing_node, is_timed_out) => {
                        println!("Node {} received ping response from Node {}", &id, from);
                        match probing_node {
                            Some(n) => {
                                DefaultMemberNode::send_to(n.id, Message::ProbeResponse(from, is_timed_out), &connection);
                            }
                            None => {
                                let mut node = node_ref.lock().unwrap();
                                if is_timed_out {
                                    node.set_member_node_state(from, MemberNodeState::Failed);
                                    for m_id in node.get_random_nodes(3).iter() {
                                        DefaultMemberNode::send_to(*m_id, Message::ProbeRequest(node.serialize(), from), &connection);
                                    }
                                } else {
                                    node.set_member_node_state(from, MemberNodeState::Alive)
                                }
                            }
                        }
                    }
                    Message::ProbeRequest(from, timed_out_node) => {
                        let node = node_ref.lock().unwrap();
                        DefaultMemberNode::send_to(timed_out_node, Message::Ping(node.serialize(), Option::Some(from)), &connection);
                    }
                    Message::ProbeResponse(from, is_timed_out) => {
                        if is_timed_out.not() {
                            node_ref.lock().unwrap().set_member_node_state(from, MemberNodeState::Alive);
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
                match node.get_random_node() {
                    Some(m_id) => {
                        DefaultMemberNode::send_to(*m_id, Message::Ping(node.serialize(), Option::None), &connection_ref);
                    }
                    None => {}
                }
            }
        });
        node_details
    }

    fn get_random_nodes(&self, number: usize) -> Vec<u16> {
        self.details.members.get_random_nodes(number)
    }

    fn send_to(id: u16, message: Message, connection_factory: &Arc<Mutex<ConnectionFactory>>) {
        connection_factory.lock().unwrap().send_to(id, message);
    }

    pub fn add_member_node(&mut self, id: u16) {
        self.details.members.add(id);
    }

    pub fn add_member_nodes(&mut self, members: &MemberNodesRegistry) {
        self.details.members.add_all(self.details.id, members);
    }

    fn get_random_node(&self) -> Option<&u16> {
        self.details.members.get_random_node()
    }

    fn serialize(&self) -> MemberNodeDetails {
        self.details.serialize()
    }

    fn set_state(&mut self, state: MemberNodeState) {
        self.details.change_state(state);
    }

    pub fn set_member_node_state(&mut self, member_node_id: u16, state: MemberNodeState) {
        self.details.members.set_node_state(member_node_id, state);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemberNodeState {
    Alive,
    Suspected,
    Failed,
}

impl Display for MemberNodeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct MemberNodeDetails {
    id: u16,
    state: MemberNodeState,
    members: MemberNodesRegistry,
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
            members: MemberNodesRegistry::new(),
        }
    }

    pub fn id(&self) -> u16 { self.id }

    pub fn name(&self) -> String { format!("node-{}", self.id) }

    pub fn state(&self) -> &MemberNodeState { &self.state }

    pub fn change_state(&mut self, state: MemberNodeState) { self.state = state }

    pub fn serialize(&self) -> MemberNodeDetails {
        let mut new_members = HashMap::new();
        let members = &self.members.members;
        for m in members {
            new_members.insert(*m.0, *m.1);
        }
        MemberNodeDetails {
            id: self.id,
            state: self.state.clone(),
            members: MemberNodesRegistry {
                members: new_members
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct MemberNodesRegistry {
    members: HashMap<u16, MemberNodeState>,
}

impl MemberNodesRegistry {
    pub fn new() -> Self {
        MemberNodesRegistry {
            members: HashMap::new()
        }
    }

    pub fn add(&mut self, id: u16) {
        self.members.insert(id, MemberNodeState::Alive);
    }

    pub fn add_all(&mut self, self_id: u16, members: &MemberNodesRegistry) {
        for id in members.members.keys().filter(|i| **i != self_id) {
            match members.members.get(id) {
                Some(state) => {
                    self.members.insert(*id, *state);
                }
                None => {}
            }
        }
    }

    pub fn set_node_state(&mut self, id: u16, state: MemberNodeState) {
        self.members.insert(id, state);
    }

    pub fn get_state_for(&self, id: u16) -> Option<&MemberNodeState> {
        self.members.get(&id)
    }

    pub fn get_random_node(&self) -> Option<&u16> {
        if self.is_empty() {
            None
        } else {
            let members: Vec<&u16> = self.members.keys()
                .filter(|id| *self.members.get(id).unwrap() == MemberNodeState::Alive)
                .collect();

            let random_index = thread_rng().gen_range(0..members.len());
            let random_node = &members[random_index];
            Some(random_node)
        }
    }

    pub fn get_random_nodes(&self, number: usize) -> Vec<u16> {
        use rand::prelude::*;
        let mut members: Vec<&u16> = self.members.keys()
            .filter(|id| *self.members.get(id).unwrap() == MemberNodeState::Alive)
            .collect();

        members.shuffle(&mut rand::thread_rng());
        members.iter().take(number).cloned().cloned().collect()
    }

    fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

impl Display for MemberNodesRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for id in self.members.keys() {
            s = s.add(format!("{} - {}; ", id, self.members.get(id).unwrap()).as_str());
        }
        write!(f, "{:?}", s)
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
            Some(c) => {
                c.send(message).map_err(|err| println!("{:?}", err));
            }
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