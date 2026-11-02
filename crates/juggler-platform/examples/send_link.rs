//! A walk's check of `apps::send_link` on a real app: sends one link to
//! one process. `cargo run -p juggler-platform --example send_link -- <pid> <link>`

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(pid), Some(link)) = (args.next().and_then(|pid| pid.parse().ok()), args.next()) else {
        eprintln!("usage: send_link <pid> <link>");
        std::process::exit(2);
    };
    match juggler_platform::apps::send_link(pid, &link) {
        Ok(()) => println!("sent"),
        Err(error) => {
            eprintln!("not sent: {error}");
            std::process::exit(1);
        }
    }
}
