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

## Usage
```bash
# See child task details
# phab task detail <task number> --api-token <token> --host <host>
phab task detail 22557 \
  --api-token my-token \
  --host="yourphabricatorhost.com" \
  --pkcs12-path="<optional /path/to/pkcs12file>" \
  --pkcs12-password="<required if pkcs12-path is set>"
  --print-json # Optional, set if you want to print output as raw json
```
