# identity-cloud-agent

[![Build Status](https://github.com/chriamue/identity-cloud-agent/actions/workflows/coverage.yml/badge.svg)](https://github.com/chriamue/identity-cloud-agent/actions)
[![codecov](https://codecov.io/gh/chriamue/identity-cloud-agent/branch/main/graph/badge.svg?token=QEH2EW6LX4)](https://codecov.io/gh/chriamue/identity-cloud-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Cloud Agent for IOTA Idendity

## quickstart

Configure the config file. Set the stronghold_path.

```toml
# Rocket.toml
[default]
ident = "identity-cloud-agent"
key_seed = "BHyHWQqKvvgbcGoXiGS33iUu1Q4KGKP4pJK11RNWzr8c"
did_key = "did:key:z6LSgPAyaBFBaEDkUVdN68WRDVZJevc1nNi9G675oK1NsEXN"
did_iota = "did:iota:As1FSRYahR2JYi3EyvWan43pLrnjGLkDffwQDcBf545G"
wallet_path = "wallet.hold.example"
wallet_password = "changeme"
webhook_url = "http://localhost:8000"

[debug]
port = 8000
ext_hostname = "http://localhost:8000"
ext_service = "http://localhost:8000"

[release]
address = "0.0.0.0"
port = 8080
```

Now start using cargo.

```sh
cargo run
```

A new identity will be created and the did will be printed.
Stop the agent and change the did in the config file.
Start the agent again.

Visit http://localhost:8000 which redirects to the swagger-ui.

## docker

Run two ica using docker compose command

```sh
docker compose --profile second-ica up
```

Visit first ica on http://localhost:8080 and second on http://localhost:8090 .

## example did doc

https://explorer.iota.org/mainnet/identity-resolver/did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf

```json
{
    "id": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf",
    "verificationMethod": [
        {
            "id": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf#kex-0",
            "controller": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf",
            "type": "X25519KeyAgreementKey2019",
            "publicKeyMultibase": "z2vnrjirdCJdt3zSqSs1v4yUHxQathWPcWkA7ESBntCpv"
        }
    ],
    "capabilityInvocation": [
        {
            "id": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf#sign-0",
            "controller": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf",
            "type": "Ed25519VerificationKey2018",
            "publicKeyMultibase": "zCnWRX8zfqRmuTci7Hz3QWn2HxQUNkpKDmR3L2FfVueut"
        }
    ],
    "service": [
        {
            "id": "did:iota:6Xbu1cFwkhL6WgmAyLNoWmYqS5b17nrVefUtLn1dHhbf#endpoint",
            "type": "Endpoint",
            "serviceEndpoint": "http://ica2:8090/"
        }
    ]
}

```
