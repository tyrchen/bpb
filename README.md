# bpb - boats's personal barricade

A minimal Git commit signing tool that replaces GPG for signing purposes. Uses ed25519 keys for fast, secure signatures with OpenPGP-compatible output.

## Features

- Ed25519 elliptic curve signatures (stronger and faster than RSA)
- Multiple named identities for different contexts (work, personal, opensource)
- Automatic identity selection based on git config
- OpenPGP-compatible output (works with GitHub signature verification)
- External secret storage support (pass, 1Password, custom scripts)
- Transparent GPG fallback for unsupported operations

## Installation

Install from GitHub:

```bash
cargo install --git https://github.com/withoutboats/bpb
```

## Quick Start

### 1. Create your first identity

```bash
bpb init "Your Name <your.email@example.com>"
```

This creates `~/.bpb_keys.toml` with your keypair and prints your public key in OpenPGP format.

### 2. Configure Git to use bpb

```bash
git config --global gpg.program bpb
git config --global commit.gpgsign true
```

### 3. Add your public key to GitHub

```bash
bpb print
```

Copy the output and add it to GitHub at Settings → SSH and GPG keys → New GPG key.

## Commands

### Identity Management

| Command | Description |
|---------|-------------|
| `bpb init <userid>` | Create a new identity (use `--id <name>` for named identity) |
| `bpb list` | List all configured identities |
| `bpb print` | Print public key in OpenPGP format (use `--id <name>` for specific identity) |
| `bpb default <name>` | Set the default identity |
| `bpb rename <old> <new>` | Rename an identity |
| `bpb remove <name>` | Remove an identity |
| `bpb export <name>` | Export identity as TOML (add `--secret` to include private key) |

### Signing

| Command | Description |
|---------|-------------|
| `bpb --sign` or `bpb -s` | Sign data from stdin (use `--id <name>` for specific identity) |

## Multi-Identity Support

Create separate identities for different contexts:

```bash
# Create identities
bpb init "Personal <me@personal.dev>"
bpb init --id work "Work Name <me@company.com>"
bpb init --id oss "OSS Name <me@opensource.org>"

# List all identities
bpb list
# Output:
#   default - Personal <me@personal.dev> (default)
#   work - Work Name <me@company.com>
#   oss - OSS Name <me@opensource.org>

# Change default
bpb default work
```

### Identity Selection Priority

When signing, bpb selects identity in this order:

1. `--id <name>` command-line flag
2. `BPB_IDENTITY` environment variable
3. Repository-local git config: `bpb.identity`
4. Global git config: `bpb.identity`
5. Default identity from config file

### Per-Repository Identity

Set different identities for different repositories:

```bash
cd ~/work/project
git config bpb.identity work

cd ~/personal/project
git config bpb.identity default
```

### Override Per-Commit

```bash
BPB_IDENTITY=oss git commit -m "contribution"
```

## Configuration

Configuration is stored at `~/.bpb_keys.toml` (override with `BPB_KEYS` environment variable).

### Example Configuration

```toml
version = 2
default = "personal"

[identities.personal]
userid = "Alice <alice@personal.dev>"
timestamp = 1234567890
public_key = "abcd1234..."

[identities.personal.secret]
key = "efgh5678..."

[identities.work]
userid = "Alice <alice@work.com>"
timestamp = 1234567891
public_key = "ijkl9012..."

[identities.work.secret]
program = "op read 'op://Private/bpb-work/key'"
```

### External Secret Storage

Instead of storing private keys directly, you can use external programs:

```toml
[identities.work.secret]
program = "pass show bpb/work"
```

```toml
[identities.work.secret]
program = "op read 'op://Private/bpb-work/key'"
```

The program should output the hex-encoded secret key to stdout.

## GPG Compatibility

bpb acts as a drop-in replacement for GPG signing:

- Recognizes GPG signing arguments (`--sign`, `-s`, `-bsau`, etc.)
- Outputs signatures in OpenPGP format
- Delegates unrecognized commands to the real `gpg` binary

This means verification and other GPG operations still work through the system GPG.

## License

MIT OR Apache-2.0
