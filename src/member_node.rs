pub mod swim_node {
    use crate::connection::swim_node::ConnectionFactory;
    use std::{thread};
    use std::collections::{HashMap};
    use std::fmt::{Display, Formatter};
    use std::ops::{Add, Not};
    use std::sync::{Arc, mpsc, Mutex};
    use std::sync::mpsc::{Receiver, Sender};
    use std::time::Duration;
    use rand;
    use rand::{Rng, thread_rng};
    use crate::message::swim_node::Message;

    pub trait MemberNode {
        fn host(&self) -> u16;

        fn serialize(&self) -> MemberNodeDetails;
    }

    pub struct DefaultMemberNode {
        details: MemberNodeDetails,
    }

    impl MemberNode for DefaultMemberNode {
        fn host(&self) -> u16 {
            self.details.host
        }

        fn serialize(&self) -> MemberNodeDetails {
            self.details.serialize()
        }
    }

    impl DefaultMemberNode {
        pub fn new(host: u16, connection: Arc<Mutex<ConnectionFactory>>) -> Arc<Mutex<MemberNodeDetails>> {
            let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
            let connection_ref = Arc::clone(&connection);
            connection.lock().unwrap().add_connection(host, sender);

            let node_details = Arc::new(Mutex::new(MemberNodeDetails::new(host)));
            let node = DefaultMemberNode {
                details: MemberNodeDetails::new(host)
            };
            let node_ref = Arc::new(Mutex::new(node));
            let node_ref_2 = Arc::clone(&node_ref);

            thread::spawn(move || {
                println!("Node {} started to listen requests", &host);
                loop {
                    match receiver.recv().unwrap() {
                        Message::Request(from, data) => {
                            println!("Node {} received message: {}", host, data);

                            let mut node = node_ref.lock().unwrap();
                            node.add_member_node(from.host);

                            DefaultMemberNode::send_to(from.host, Message::Response(node.serialize(), String::from("hi")), &connection);
                        }
                        Message::Response(from, data) => {
                            let mut node = node_ref.lock().unwrap();
                            node.add_member_node(from.host);

                            println!("Node {} received response from Node {}: {}", host, from.host, data)
                        }
                        Message::Ping(from, probing_node) => {
                            let mut node = node_ref.lock().unwrap();
                            node.add_member_nodes(&from.members);

                            println!("Node {} received ping request from Node {}, with members: {}", &host, from.host, node.details.members);

                            DefaultMemberNode::send_to(from.host, Message::PingResponse(host, probing_node, false), &connection);
                        }
                        Message::PingResponse(from, probing_node, is_timed_out) => {
                            println!("Node {} received ping response from Node {}", &host, from);
                            match probing_node {
                                Some(n) => {
                                    DefaultMemberNode::send_to(n.host, Message::ProbeResponse(from, is_timed_out), &connection);
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
                            println!("Node {} received termination message", &host);
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

        fn send_to(host: u16, message: Message, connection_factory: &Arc<Mutex<ConnectionFactory>>) {
            connection_factory.lock().unwrap().send_to(host, message);
        }

        fn get_random_nodes(&self, number: usize) -> Vec<u16> {
            self.details.members.get_random_nodes(number)
        }

        pub fn add_member_node(&mut self, host: u16) {
            self.details.members.add(host);
        }

        pub fn add_member_nodes(&mut self, members: &MemberNodesRegistry) {
            self.details.members.add_all(self.details.host, members);
        }

        fn get_random_node(&self) -> Option<&u16> {
            self.details.members.get_random_node()
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
        host: u16,
        state: MemberNodeState,
        members: MemberNodesRegistry,
    }

    impl MemberNodeDetails {
        pub fn new(host: u16) -> Self {
            MemberNodeDetails {
                host: host,
                state: MemberNodeState::Alive,
                members: MemberNodesRegistry::new(),
            }
        }

        pub fn host(&self) -> u16 { self.host }

        pub fn name(&self) -> String { format!("node-{}", self.host) }

        pub fn state(&self) -> &MemberNodeState { &self.state }

        pub fn change_state(&mut self, state: MemberNodeState) { self.state = state }

        pub fn serialize(&self) -> MemberNodeDetails {
            let mut new_members = HashMap::new();
            let members = &self.members.members;
            for m in members {
                new_members.insert(*m.0, *m.1);
            }
            MemberNodeDetails {
                host: self.host,
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

        pub fn add(&mut self, host: u16) {
            self.members.insert(host, MemberNodeState::Alive);
        }

        pub fn add_all(&mut self, self_id: u16, members: &MemberNodesRegistry) {
            for host in members.members.keys().filter(|i| **i != self_id) {
                match members.members.get(host) {
                    Some(state) => {
                        self.members.insert(*host, *state);
                    }
                    None => {}
                }
            }
        }

        pub fn set_node_state(&mut self, host: u16, state: MemberNodeState) {
            self.members.insert(host, state);
        }

        pub fn get_state_for(&self, host: u16) -> Option<&MemberNodeState> {
            self.members.get(&host)
        }

        pub fn get_random_node(&self) -> Option<&u16> {
            if self.is_empty() {
                None
            } else {
                let members: Vec<&u16> = self.members.keys()
                    .filter(|host| *self.members.get(host).unwrap() == MemberNodeState::Alive)
                    .collect();

                let random_index = thread_rng().gen_range(0..members.len());
                let random_node = &members[random_index];
                Some(random_node)
            }
        }

        pub fn get_random_nodes(&self, number: usize) -> Vec<u16> {
            use rand::prelude::*;
            let mut members: Vec<&u16> = self.members.keys()
                .filter(|host| *self.members.get(host).unwrap() == MemberNodeState::Alive)
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
            for host in self.members.keys() {
                s = s.add(format!("{} - {}; ", host, self.members.get(host).unwrap()).as_str());
            }
            write!(f, "{:?}", s)
        }
    }
}