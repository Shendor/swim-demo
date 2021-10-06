use std::ops::{Not, Range};
use std::sync::mpsc::{Receiver, Sender};
use rand;
use rand::{Rng, thread_rng};
use std::{thread, vec};
use std::rc::Rc;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

pub enum Message {
    DATA(String),
    PING(u16),
    SHUTDOWN(),
}

pub trait MemberNode {
    fn send(&self, message: Message);

    fn shut_down(&self);
}

pub struct DefaultMemberNode {
    details: MemberNodeDetails,
    sender: Sender<Message>,
    // receiver: Arc<Mutex<Receiver<Message>>>,
    listener: JoinHandle<()>,
}

impl MemberNode for DefaultMemberNode {
    fn send(&self, message: Message) {
        println!("Node {} sent message", self.details.id);
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
        let listener = thread::spawn(move || {
            println!("Node {} started to listen requests", &id);
            let mut is_terminated = false;
            while is_terminated.not() {
                match receiver.recv().unwrap() {
                    Message::DATA(d) => println!("Node {} received message: {}", id, d),
                    Message::PING(from) => println!("Node {} received ping request from Node {}", id, from),
                    Message::SHUTDOWN() => {
                        println!("Node {} received termination message", id);
                        is_terminated = true
                    }
                }
            }
        });

        DefaultMemberNode {
            details: MemberNodeDetails::new(id),
            sender,
            listener,
        }
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

pub struct MemberNodes {
    members: Vec<MemberNodeDetails>,
}

impl MemberNodes {
    pub fn new(node: MemberNodeDetails) -> Self {
        MemberNodes {
            members: vec!(node)
        }
    }

    pub fn get_random_node(&self) -> Option<&MemberNodeDetails> {
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