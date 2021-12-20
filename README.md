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

stronghold_path = "account-stronghold.hodl"
password = "changeme"
endpoint = "http://localhost:8000"
webhook_url = "http://localhost:8000"
did = "did:iota:6HnYPKwSAzf3yRLtkWN7uAUHEf8cCAfdyRSK1EJXSaUU"

[debug]
port = 8000
ext_hostname = "http://localhost:8000"

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
