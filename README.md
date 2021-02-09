ACME DNS Client
===============

ACME (Let's Encrypt) client for issuing SSL/TLS certificate through DNS validation.

## Motivation
To order a wildcard SSL/TLS certificate (i.e. a certificate for `*.example.com`) from an ACME provider like Let's Encrypt, you are required to perform DNS validation to prove ownership of the domain.

This requires you to serve a TXT record at `_acme-challenge.example.com` during validation containing a token provided to you by the ACME provider. However, automating this can be challenging as many domain name registrars do not provide a way to update records programmatically.

Fortunately, with a particular set of resource records that you pre-configure with your domain registrar, you can defer all future DNS queries for `_acme-challenge.example.com` to your server of choice. This tool will let you take advantage of this by running a temporary barebones DNS server that only responds to validation queries from the ACME provider.


## Installation
Download the binary from GitHub by running `wget https://github.com/ShazzAmin/acme-dns-client/releases/download/v0.2.0/acme-dns-client`.

Alternatively, you can build from source by cloning this repository and running `cargo build --release`.


## Use
Ensure your DNS records are set-up as such with your domain name registrar:
```
_acme-challenge.example.com.     CNAME     _acme-challenge.acme-dns-server.example.com.
acme-dns-server.example.com.     NS        yourserver.example.com.
yourserver.example.com           A         <IP address of the server you will run this tool on>
```

Now simply run the tool:
```shell
$ sudo ./acme-dns-client --domain "*.example.com" --email "hello@example.com" --output "/etc/acme-certs/"
Ordering certificate for *.example.com using email hello@example.com from Let's Encrypt...
DNS validation required; starting DNS server...
DNS validation complete.
Order successful; saved certificate and private keys in /etc/acme-certs/
Certificate will expire in 89 days.
```

If everything goes well, there should be 1 `.crt` file (your public certificate) and 2 `.key` files (one is the private key for your certificate and the other is the private key for your ACME account) in `/etc/acme-certs/`.

You can set this up as a cron job to renew your certificate periodically.

### Testing
Use the `--staging` option if you are testing. This will use the [staging Let's Encrypt provider](https://letsencrypt.org/docs/staging-environment/).


## License
[MIT](LICENSE)

Copyright (c) 2020 Shazz Amin
