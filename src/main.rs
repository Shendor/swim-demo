use std::thread;
use std::time::Duration;
use swim_app::member_node::{MemberNode, MemberNodeDetails};
use swim_app::run_network;

fn main() {
    // let a = MemberNodes::new(MemberNodeDetails::new(1));
    // println!("node = {}", a.get_random_node().unwrap().id())

    let mut router = run_network();
    router.send_to(2, 1);
    // router.send_to(3, 1);
    // router.send_to(3, 2);
    thread::sleep(Duration::from_secs(10));
    router.shut_down();
}
