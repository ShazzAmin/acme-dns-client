#[macro_use] extern crate packed_struct_codegen;

mod dns;
mod persist;

use acme_lib::{Directory, DirectoryUrl, create_p384_key};
use clap::{Arg, App};
use persist::FilePersist;

fn main() {
    let args = App::new("ACME DNS Client")
        .version("0.1.0")
        .author("Shazz Amin <me@shazz.me>")
        .about("ACME (Let's Encrypt) client for issuing (wildcard) SSL/TLS certificate through DNS validation")
        .arg(Arg::new("domain")
            .short('d')
            .long("domain")
            .value_name("DOMAIN")
            .about("Domain to issue certificate for; e.g.: *.example.com")
            .required(true)
            .takes_value(true))
        .arg(Arg::new("email")
            .short('e')
            .long("email")
            .value_name("EMAIL")
            .about("E-mail address to use for associated account; e.g.: hello@example.com")
            .required(true)
            .takes_value(true))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("DIRECTORY")
            .about("Directory to store certificate and private keys in; e.g.: /etc/acme-certs/")
            .required(true)
            .takes_value(true))
        .arg(Arg::new("staging")
            .short('s')
            .long("staging")
            .about("Use the staging Let's Encrypt certificate authority (for testing)"))
        .get_matches();

    let staging = args.is_present("staging");
    let domain = args.value_of("domain").unwrap();
    let email = args.value_of("email").unwrap();
    let output_directory = args.value_of("output").unwrap();

    println!("Ordering certificate for {} using email {} from Let's Encrypt{}...",
              domain,
              email,
              if staging { " (staging)" } else { "" });

    let url = if staging { DirectoryUrl::LetsEncryptStaging } else { DirectoryUrl::LetsEncrypt };

    let mut order = Directory::from_url(FilePersist::new(output_directory, domain), url).unwrap()
                     .account(email).unwrap()
                     .new_order(domain, &[]).unwrap();

    let csr = loop {
        if let Some(csr) = order.confirm_validations() {
            break csr;
        }

        let challenge = order.authorizations().unwrap()[0].dns_challenge();

        println!("DNS validation required; starting DNS server...");

        let dns_kill_tx = dns::start_responding_with(challenge.dns_proof());
        challenge.validate(5000).unwrap();
        dns_kill_tx.send(()).unwrap();

        println!("DNS validation complete.");

        order.refresh().unwrap();
    };

    let certificate = csr.finalize_pkey(create_p384_key(), 5000).unwrap().download_and_save_cert().unwrap();
    println!("Order successful; saved certificate and private keys in {}", output_directory);
    println!("Certificate will expire in {} days.", certificate.valid_days_left());
}
