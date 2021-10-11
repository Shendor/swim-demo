pub mod swim_node {
    use std::collections::HashMap;
    use std::sync::mpsc::Sender;
    use crate::member_node::swim_node::Message;

    pub struct ConnectionFactory {
        connection: HashMap<u16, Sender<Message>>,
    }

    impl ConnectionFactory {
        pub fn new() -> ConnectionFactory {
            ConnectionFactory {
                connection: HashMap::new()
            }
        }

        pub fn get_connection_for(&self, host: u16) -> Option<&Sender<Message>> {
            self.connection.get(&host)
        }

        pub fn send_to(&self, host: u16, message: Message) {
            match self.connection.get(&host) {
                Some(c) => {
                    c.send(message).map_err(|err| println!("{:?}", err));
                }
                _ => {}
            }
        }

        pub fn add_connection(&mut self, host: u16, connection: Sender<Message>) {
            self.connection.insert(host, connection);
        }

        pub fn remove_connection(&mut self, host: u16) {
            self.connection.remove(&host);
        }
    }
}