# Phab
My laziness to click and track phabricator UI drives me to create this project.


## Installation
### Cargo
```bash
cargo install phab
```

### Manual
```bash
git clone git@github.com:sendyhalim/phab.git

cd phab

cargo install --path . --force
```

### Download
TODO: Dynamically Linked Binaries

## Setup
First put config `~/.phab`

```bash
# We're using hjson format
{
  api_token: phabricatortoken
  host: https://yourphabricatorhost.com
  cert_identity_config: { # This is optional
    pkcs12_path: "......"
    pkcs12_password: "....."
  }
}
```

## Usage
```bash
# See task details including its child
phab task detail 22557 \
  --print-json # Optional, set if you want to print output as raw json
```
