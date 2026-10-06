use clap::Parser;
use rm_server_sync::{Service, http};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:7878")]
    address: SocketAddr,
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..))]
    token_ttl_seconds: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    http::run(
        args.address,
        Service::new(Duration::from_secs(args.token_ttl_seconds)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_ttl_defaults_to_300_seconds() {
        let args = Args::try_parse_from(["rm-server-sync"]).unwrap();
        assert_eq!(args.token_ttl_seconds, 300);
    }

    #[test]
    fn token_ttl_rejects_zero() {
        assert!(Args::try_parse_from(["rm-server-sync", "--token-ttl-seconds", "0"]).is_err());
    }
}
