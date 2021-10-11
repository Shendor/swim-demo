use std::thread;
use std::time::Duration;
use swim_app::run_network;

fn main() {
    let mut router = run_network();
    router.send_to(2, 1);
    router.send_to(3, 1);
    router.send_to(2, 4);
    router.send_to(5, 1);
    thread::sleep(Duration::from_secs(20));
    router.shut_down();
}
