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

    const PING_DELAY: u64 = 1;
    const NUMBER_RANDOM_PROBE_NODES: usize = 3;

    pub struct DefaultMemberNode {
        details: MemberNodeDetails,
    }

    impl DefaultMemberNode {
        pub fn new(host: u16, connection: Arc<Mutex<ConnectionFactory>>) -> Arc<Mutex<DefaultMemberNode>> {
            let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel();
            let connection_ref = Arc::clone(&connection);
            connection.lock().unwrap().add_connection(host, sender);

            let node = DefaultMemberNode {
                details: MemberNodeDetails::new(host)
            };
            let node_ref = Arc::new(Mutex::new(node));
            let node_ref_2 = Arc::clone(&node_ref);
            let node_ref_3 = Arc::clone(&node_ref);

            thread::spawn(move || {
                println!("Node {} started to listen requests", &host);
                loop {
                    match receiver.recv().unwrap() {
                        Message::Request(from, data) => {
                            println!("Node {} received message: {}", host, data);

                            let mut node = node_ref.lock().unwrap();
                            node.add_member_node(from.host);

                            send_to(from.host, Message::Response(node.serialize(), String::from("hi")), &connection);
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

                            send_to(from.host, Message::PingResponse(host, probing_node, node.is_alive().not()), &connection);
                        }
                        Message::PingResponse(from, probing_node, is_timed_out) => {
                            match probing_node {
                                Some(n) => {
                                    send_to(n.host, Message::ProbeResponse(from, is_timed_out), &connection);
                                }
                                None => {
                                    let mut node = node_ref.lock().unwrap();
                                    if is_timed_out {
                                        println!("Node {} didn't received ping response from Node {}. Starting to probe it...", &host, from);
                                        node.set_member_node_state(from, MemberNodeState::Suspected);
                                        for random_host in node.get_random_nodes(NUMBER_RANDOM_PROBE_NODES).iter() {
                                            send_to(*random_host, Message::ProbeRequest(node.serialize(), from), &connection);
                                        }
                                    } else {
                                        println!("Node {} received ping response from Node {}", &host, from);
                                        node.set_member_node_state(from, MemberNodeState::Alive)
                                    }
                                }
                            }
                        }
                        Message::ProbeRequest(from, timed_out_node) => {
                            println!("Node {} probing timed-out Node {}", &host, timed_out_node);

                            let node = node_ref.lock().unwrap();
                            send_to(timed_out_node, Message::Ping(node.serialize(), Option::Some(from)), &connection);
                        }
                        Message::ProbeResponse(from, is_timed_out) => {
                            if is_timed_out.not() {
                                println!("Node {} reported back on-line", &from);
                                node_ref.lock().unwrap().set_member_node_state(from, MemberNodeState::Alive);
                            } else {
                                println!("Node {} is still off-line", &from);
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
                    thread::sleep(Duration::from_secs(PING_DELAY));
                    let node = node_ref_2.lock().unwrap();
                    if node.details.state != MemberNodeState::Failed {
                        match node.get_random_node() {
                            Some(m_id) => {
                                send_to(*m_id, Message::Ping(node.serialize(), Option::None), &connection_ref);
                            }
                            None => {}
                        }
                    }
                }
            });
            node_ref_3
        }

        pub fn details(&self) -> &MemberNodeDetails {
            &self.details
        }

        pub fn change_state(&mut self, state: MemberNodeState) { self.details.change_state(state); }

        fn is_alive(&self) -> bool {
            *self.details.state() == MemberNodeState::Alive
        }

        fn get_random_nodes(&self, number: usize) -> Vec<u16> {
            self.details.members.get_random_nodes(number)
        }

        fn add_member_node(&mut self, host: u16) {
            self.details.members.add(host);
        }

        fn add_member_nodes(&mut self, members: &MemberNodesRegistry) {
            self.details.members.add_all(self.details.host, members);
        }

        fn get_random_node(&self) -> Option<&u16> {
            self.details.members.get_random_node()
        }

        fn set_member_node_state(&mut self, member_node_id: u16, state: MemberNodeState) {
            self.details.members.set_node_state(member_node_id, state);
        }

        fn serialize(&self) -> MemberNodeDetails {
            self.details.serialize()
        }
    }

    fn send_to(host: u16, message: Message, connection_factory: &Arc<Mutex<ConnectionFactory>>) {
        connection_factory.lock().unwrap().send_to(host, message);
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
                host,
                state: MemberNodeState::Alive,
                members: MemberNodesRegistry::new(),
            }
        }

        pub fn host(&self) -> u16 { self.host }

        pub fn name(&self) -> String { format!("node-{}", self.host) }

        pub fn state(&self) -> &MemberNodeState { &self.state }

        pub fn members(&self) -> &MemberNodesRegistry { &self.members }

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
                        if *state == MemberNodeState::Failed {
                            self.members.remove(host);
                        } else {
                            self.members.insert(*host, *state);
                        }
                    }
                    None => {}
                }
            }
        }

        pub fn set_node_state(&mut self, host: u16, state: MemberNodeState) {
            match self.members.get(&host) {
                Some(m) => {
                    if state == MemberNodeState::Suspected && *m == MemberNodeState::Suspected {
                        self.members.insert(host, MemberNodeState::Failed);
                    } else {
                        self.members.insert(host, state);
                    }
                }
                None => {}
            };
        }

        pub fn get_state_for(&self, host: u16) -> Option<&MemberNodeState> {
            self.members.get(&host)
        }

        pub fn get_random_node(&self) -> Option<&u16> {
            let members: Vec<&u16> = self.members.keys()
                .filter(|host| self.is_host_not_failed(host))
                .collect();
            if members.is_empty() {
                None
            } else {
                let random_index = thread_rng().gen_range(0..members.len());
                let random_node = members[random_index];
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

        fn is_host_not_failed(&self, host: &&u16) -> bool {
            *self.members.get(host).unwrap() != MemberNodeState::Failed
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