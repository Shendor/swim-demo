#[cfg(test)]
mod tests {
    mod connection_tests {
        use std::sync::mpsc;
        use std::sync::mpsc::Receiver;
        use crate::connection::swim_node::{ConnectionFactory, ConnectionRegistry};
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
        use crate::connection::swim_node::{ConnectionFactory, ConnectionRegistry};
        use crate::member_node::swim_node::{DefaultMemberNode, MemberNodeState};
        use crate::message::swim_node::Message;

        #[test]
        fn test_member_nodes_sending_message() {
            let connection_factory = ConnectionFactory::new();
            let connection_ref = Arc::new(Mutex::new(connection_factory));
            let node1 = DefaultMemberNode::new(1, Arc::<Mutex<ConnectionFactory>>::clone(&connection_ref));
            let node2 = DefaultMemberNode::new(2, Arc::<Mutex<ConnectionFactory>>::clone(&connection_ref));

            let serialized_details = node1.lock().unwrap().details().serialize();
            connection_ref.lock().unwrap().send_to(2, Message::Request(serialized_details, String::from("hello")));

            thread::sleep(Duration::from_secs(1));

            assert_eq!(MemberNodeState::Alive, *node1.lock().unwrap().details().members().get_state_for(2).unwrap());
            assert_eq!(MemberNodeState::Alive, *node2.lock().unwrap().details().members().get_state_for(1).unwrap());
        }

        #[test]
        fn test_member_nodes_when_one_times_out() {
            let connection_factory = ConnectionFactory::new();
            let connection_ref = Arc::new(Mutex::new(connection_factory));
            let node1 = DefaultMemberNode::new(1, Arc::<Mutex<ConnectionFactory>>::clone(&connection_ref));
            let node2 = DefaultMemberNode::new(2, Arc::<Mutex<ConnectionFactory>>::clone(&connection_ref));

            let serialized_details = node1.lock().unwrap().details().serialize();
            connection_ref.lock().unwrap().send_to(2, Message::Request(serialized_details, String::from("hello")));

            node2.lock().unwrap().change_state(MemberNodeState::Failed);

            thread::sleep(Duration::from_secs(2));

            assert_eq!(MemberNodeState::Suspected, *node1.lock().unwrap().details().members().get_state_for(2).unwrap());

            thread::sleep(Duration::from_secs(2));

            assert_eq!(MemberNodeState::Failed, *node1.lock().unwrap().details().members().get_state_for(2).unwrap());
        }
    }

    mod test_router {
        use std::sync::{Arc, Mutex};
        use crate::member_node::swim_node::{MemberNode, MemberNodeDetails};
        use mockall::*;
        use mockall::predicate::*;
        use crate::connection::swim_node::{ConnectionRegistry};
        use crate::network_router::{DefaultNodeRequestRouter, NodeFactory, NodeRequestRouter};
        use std::sync::mpsc::Sender;
        use crate::message::swim_node::Message;

        mock! {
            TestMemberNode {}
            impl MemberNode for TestMemberNode {
                fn host(&self) -> u16;
                fn serialize_host_details(&self) -> MemberNodeDetails;
            }
        }

        mock! {
            TestNodeFactory {}
            impl NodeFactory<MockTestMemberNode> for TestNodeFactory {
                fn create(&self, id: u16, connection: Arc<Mutex<dyn ConnectionRegistry>>) -> Arc<Mutex<MockTestMemberNode>>;
            }
        }

        mock! {
            TestConnectionRegistry {}
            impl ConnectionRegistry for TestConnectionRegistry {
                fn send_to(&self, host: u16, message: Message);
                fn add_connection(&mut self, host: u16, connection: Sender<Message>);
                fn remove_connection(&mut self, host: u16);
            }
        }

        #[test]
        fn test_mock() {
            let mut node1 = MockTestMemberNode::new();
            node1.expect_host().returning(|| 1);
            node1.expect_serialize_host_details()
                .returning(|| MemberNodeDetails::new(1));

            let mut node_factory = MockTestNodeFactory::new();
            node_factory.expect_create()
                .withf(|host: &u16, _: &Arc<Mutex<dyn ConnectionRegistry>>| *host == 1)
                .return_const(Arc::new(Mutex::new(node1)));

            let mut node2 = MockTestMemberNode::new();
            node2.expect_host().returning(|| 2);
            node2.expect_serialize_host_details()
                .returning(|| MemberNodeDetails::new(2));

            node_factory.expect_create()
                .withf(|host: &u16, _: &Arc<Mutex<dyn ConnectionRegistry>>| *host == 2)
                .return_const(Arc::new(Mutex::new(node2)));

            let mut connection_registry = MockTestConnectionRegistry::new();
            connection_registry.expect_send_to()
                .withf(|host: &u16, message: &Message|
                    match message {
                        Message::Request(n, d) => *host == 1 && n.host() == 2 && *d == String::from("hello from 2"),
                        _ => false,
                    })
                .return_const(());

            let mut router = DefaultNodeRequestRouter::new(Box::<MockTestNodeFactory>::new(node_factory), Arc::new(Mutex::new(connection_registry)));
            router.start();
            router.send(2, 1)
        }
    }
}