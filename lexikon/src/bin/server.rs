use lexikon::start_server;
use lexikon::run_server;

pub fn main() {
    run_server().expect("Failed to start server");
}
