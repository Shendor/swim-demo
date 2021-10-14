pub mod swim_node {
    use std::collections::HashMap;
    use std::sync::mpsc::Sender;
    use crate::message::swim_node::Message;

    pub trait ConnectionRegistry : Send {
        fn get_connection_for(&self, host: u16) -> Option<&Sender<Message>>;

        fn send_to(&self, host: u16, message: Message);

        fn add_connection(&mut self, host: u16, connection: Sender<Message>);

        fn remove_connection(&mut self, host: u16);
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
    }

    impl ConnectionRegistry for ConnectionFactory {
        fn get_connection_for(&self, host: u16) -> Option<&Sender<Message>> {
            self.connection.get(&host)
        }

        fn send_to(&self, host: u16, message: Message) {
            match self.connection.get(&host) {
                Some(c) => {
                    c.send(message).unwrap_or_else(|err| println!("Failed to send message from host {} - {:?}", host, err))
                }
                _ => {}
            }
        }

        fn add_connection(&mut self, host: u16, connection: Sender<Message>) {
            self.connection.insert(host, connection);
        }

        fn remove_connection(&mut self, host: u16) {
            self.connection.remove(&host);
        }
    }
}