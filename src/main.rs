use std::thread;
use std::time::Duration;
use swim_app::run_network;

fn main() {
    let mut router = run_network();
    router.send(2, 1);
    router.send(3, 1);
    router.send(2, 4);
    // router.send_to(5, 1);
    thread::sleep(Duration::from_secs(30));
    router.shut_down();
}
