#[cfg(test)]
mod tests {
    mod connection_tests {
        use std::sync::mpsc;
        use std::sync::mpsc::Receiver;
        use crate::connection::swim_node::ConnectionFactory;
        use crate::message::swim_node::Message;

        #[test]
        fn test_connection_create() {
            let connection_factory = create_simple_connection_factory_with_receiver();

            assert!(connection_factory.0.get_connection_for(1).is_some())
        }

        #[test]
        fn test_connection_delete() {
            let mut connection_factory = create_simple_connection_factory_with_receiver();
            connection_factory.0.remove_connection(1);

            assert!(connection_factory.0.get_connection_for(1).is_none())
        }

        #[test]
        fn test_connection_send_and_receive() {
            let connection_factory = create_simple_connection_factory_with_receiver();

            connection_factory.0.send_to(1, Message::Shutdown());

            match connection_factory.1.recv().unwrap() {
                Message::Shutdown() => {}
                _ => { panic!("Failed to send message from connection factory") }
            }
        }

        fn create_simple_connection_factory_with_receiver() -> (ConnectionFactory, Receiver<Message>) {
            let channel = mpsc::channel();
            let mut connection_factory = ConnectionFactory::new();
            connection_factory.add_connection(1, channel.0);

            (connection_factory, channel.1)
        }
    }

    mod member_node_tests {
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;
        use crate::connection::swim_node::ConnectionFactory;
        use crate::member_node::swim_node::{DefaultMemberNode, MemberNodeState};
        use crate::message::swim_node::Message;

        #[test]
        fn test_member_nodes_sending_message() {
            let connection_factory = ConnectionFactory::new();
            let connection_ref = Arc::new(Mutex::new(connection_factory));
            let node1 = DefaultMemberNode::new(1, Arc::clone(&connection_ref));
            let node2 = DefaultMemberNode::new(2, Arc::clone(&connection_ref));

            let serialized_details = node1.lock().unwrap().details().serialize();
            connection_ref.lock().unwrap().send_to(2, Message::Request(serialized_details, String::from("hello")));

            thread::sleep(Duration::from_secs(1));

            assert_eq!(MemberNodeState::Alive, *node1.lock().unwrap().details().members().get_state_for(2).unwrap());
            assert_eq!(MemberNodeState::Alive, *node2.lock().unwrap().details().members().get_state_for(1).unwrap());
        }
    }
}