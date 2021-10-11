use std::thread;
use std::time::Duration;
use swim_app::run_network;

fn main() {
    let mut router = run_network();
    router.send(2, 1);
    router.send(3, 1);
    router.send(4, 2);
    router.send(5, 3);
    router.send(6, 1);
    thread::sleep(Duration::from_secs(30));
    router.shut_down();
}
