# RFC 0001: Multiple Identity Support for BPB

## Summary

Add support for managing multiple signing identities in BPB, allowing users to maintain separate keypairs for different contexts (e.g., personal vs. work, different open source projects) while preserving backward compatibility with the existing single-identity configuration.

## Motivation

Currently, BPB only supports a single keypair stored in `~/.bpb_keys.toml`. This limitation forces users who need to sign commits with different identities to either:

1. Manually swap configuration files
2. Use environment variables (`BPB_KEYS`) to point to different files
3. Fall back to GPG for multi-identity scenarios

Users commonly need multiple identities for:
- Separating personal and work commits
- Contributing to different organizations with different email addresses
- Maintaining separate identities for different open source projects
- Compliance requirements that mandate specific signing keys per project

## Design Overview

### Core Concepts

1. **Identity**: A named keypair with associated user ID (name + email)
2. **Default Identity**: The identity used when no specific identity is requested
3. **Identity Selection**: Mechanism to choose which identity to use for signing

### Configuration File Format

#### New Format (v2)

The new configuration will use a versioned format with support for multiple identities:

```toml
version = 2
default = "personal"  # Optional: name of default identity

[identities.personal]
userid = "Alice <alice@personal.dev>"
timestamp = 1703001234

[identities.personal.public]
key = "abcd1234..."

[identities.personal.secret]
key = "efgh5678..."
# OR
# program = "pass show bpb/personal"

[identities.work]
userid = "Alice Smith <alice.smith@company.com>"
timestamp = 1703002345

[identities.work.public]
key = "ijkl9012..."

[identities.work.secret]
program = "op read 'op://Private/bpb-work/secret-key'"
```

#### Legacy Format (v1) - Backward Compatible

The existing format will continue to be supported:

```toml
[public]
key = "abcd1234..."
userid = "Alice <alice@example.com>"
timestamp = 1703001234

[secret]
key = "efgh5678..."
```

When loading a v1 config, BPB will treat it as a single identity named "default".

### CLI Interface

#### New Commands

```bash
# Identity management
bpb init <userid>                    # Create default identity (backward compatible)
bpb init --id <name> <userid>        # Create named identity
bpb list                             # List all identities
bpb print                            # Print default identity's public key
bpb print --id <name>                # Print specific identity's public key
bpb remove <name>                    # Remove an identity
bpb rename <old-name> <new-name>     # Rename an identity
bpb default <name>                   # Set default identity
bpb export <name> [--secret]         # Export identity (public key only by default)
bpb import <name> <file>             # Import identity from file

# Signing (existing behavior preserved)
bpb --sign                           # Sign with default identity
bpb --sign --id <name>               # Sign with specific identity
```

#### Environment Variables

```bash
BPB_KEYS=/path/to/config.toml       # Config file location (existing)
BPB_IDENTITY=work                    # Override default identity for this session
```

### Git Integration

#### Per-Repository Identity Selection

Users can configure repository-specific identities via git config:

```bash
# Set identity for current repository
git config bpb.identity work

# Set identity globally
git config --global bpb.identity personal

# Per-directory configuration via .git/config
[bpb]
    identity = work
```

BPB will check for identity in this order:
1. `--id` command-line argument
2. `BPB_IDENTITY` environment variable
3. Repository-local git config (`bpb.identity`)
4. Global git config (`bpb.identity`)
5. Default identity from config file
6. First identity in config (if no default set)

### Implementation Plan

#### Phase 1: Core Infrastructure (Non-Breaking)

1. **Config Module Refactoring**
   - Add `ConfigVersion` enum
   - Create `Identity` struct to encapsulate keypair + metadata
   - Add `MultiConfig` struct for v2 format
   - Implement config version detection and migration

   ```rust
   pub enum ConfigVersion {
       V1,  // Legacy single-identity
       V2,  // Multi-identity
   }

   pub struct Identity {
       pub name: String,
       pub userid: String,
       pub timestamp: u64,
       pub public_key: String,
       pub secret: SecretKeySource,
   }

   pub enum SecretKeySource {
       Inline(String),
       Program(String),
   }

   pub struct MultiConfig {
       pub version: u8,
       pub default: Option<String>,
       pub identities: HashMap<String, Identity>,
   }
   ```

2. **Backward Compatibility Layer**
   - Auto-detect config version on load
   - Transparently handle v1 configs as single "default" identity
   - Migration command: `bpb migrate` to convert v1 → v2

#### Phase 2: Identity Management Commands

3. **List Command**
   ```rust
   fn list_identities() -> Result<()>
   ```
   Output format:
   ```
   * personal (default)
     Alice <alice@personal.dev>
     Fingerprint: ABCD 1234 5678 ...

     work
     Alice Smith <alice.smith@company.com>
     Fingerprint: IJKL 9012 3456 ...
   ```

4. **Identity CRUD Operations**
   - `init --id <name>`: Create new identity
   - `remove <name>`: Remove identity (with confirmation)
   - `rename <old> <new>`: Rename identity
   - `default <name>`: Set default identity

#### Phase 3: Identity Selection

5. **Git Config Integration**
   - Read `bpb.identity` from git config
   - Implement identity resolution chain

6. **Signing with Identity Selection**
   - Add `--id` flag to signing operations
   - Respect `BPB_IDENTITY` environment variable

#### Phase 4: Import/Export

7. **Export Command**
   ```bash
   bpb export personal > personal.pub
   bpb export personal --secret > personal.key  # Requires confirmation
   ```

8. **Import Command**
   ```bash
   bpb import work work-key.toml
   ```

### File Structure

```
src/
├── main.rs              # CLI entry point
├── cli.rs               # Argument parsing (new)
├── config/
│   ├── mod.rs           # Config module root
│   ├── v1.rs            # Legacy config support
│   ├── v2.rs            # New multi-identity config
│   └── migration.rs     # v1 → v2 migration
├── identity.rs          # Identity management (new)
├── key_data.rs          # Key operations (refactored)
├── git.rs               # Git integration (new)
└── tests.rs
```

### Security Considerations

1. **Secret Key Protection**
   - Secret keys remain stored with same security model
   - Program-based secrets continue to work per-identity
   - Export of secret keys requires explicit `--secret` flag and confirmation

2. **Identity Verification**
   - On load, verify public/secret key pairs match
   - Warn if identity userid doesn't match git user.email

3. **File Permissions**
   - Config file permissions checked on load (warn if too permissive)
   - Recommend 600 permissions for config file

### Migration Path

1. **Automatic Detection**: BPB detects config version automatically
2. **Seamless Upgrade**: v1 configs work without modification
3. **Explicit Migration**: `bpb migrate` converts v1 → v2
4. **No Downgrade**: v2 configs cannot be converted back to v1

### Testing Strategy

1. **Unit Tests**
   - Config parsing for both v1 and v2
   - Identity CRUD operations
   - Identity resolution chain

2. **Integration Tests**
   - Full signing workflow with multiple identities
   - Git config integration
   - Migration from v1 to v2

3. **Compatibility Tests**
   - Verify v1 configs still work
   - Verify signatures are compatible with GPG verification

### Future Considerations

1. **Key Rotation**: Add support for identity key rotation with grace period
2. **Identity Aliases**: Allow multiple names for same identity
3. **Team Sharing**: Export/import identities for team onboarding
4. **Hardware Keys**: Support for hardware security modules (YubiKey, etc.)
5. **Key Expiration**: Optional expiration dates for identities

### Alternatives Considered

1. **Separate Config Files Per Identity**
   - Rejected: More complex to manage, harder to list all identities

2. **XDG Directory Structure**
   - Rejected: Breaking change, though could be added as option later

3. **SQLite Database**
   - Rejected: Overkill for simple key storage, harder to inspect/backup

### Appendix: Example Workflows

#### Workflow 1: Setting Up Work/Personal Identities

```bash
# Initial setup (creates default identity)
$ bpb init "Alice <alice@personal.dev>"
# Converts to v2 format automatically when adding second identity

# Add work identity
$ bpb init --id work "Alice Smith <alice.smith@company.com>"

# List identities
$ bpb list
* default
  Alice <alice@personal.dev>

  work
  Alice Smith <alice.smith@company.com>

# Set work as default
$ bpb default work

# Configure work repos to use work identity
$ cd ~/work/project
$ git config bpb.identity work
```

#### Workflow 2: Per-Repository Configuration

```bash
# In personal project
$ cd ~/personal/my-project
$ git config bpb.identity personal

# In work project
$ cd ~/work/company-project
$ git config bpb.identity work

# Commits automatically use correct identity
$ git commit -S -m "Fix bug"  # Uses identity from git config
```

#### Workflow 3: One-Off Signing with Different Identity

```bash
# Usually use personal, but need to sign one commit as work
$ BPB_IDENTITY=work git commit -S -m "Company contribution"

# Or explicitly
$ echo "data" | bpb --sign --id work
```
