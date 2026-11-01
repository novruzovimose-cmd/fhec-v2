# FHEC V2: Git and CI handoff

The archived Windows V2 project has no `.git` directory. The new GitHub workflow is source-only until the owner chooses a remote and authorizes a push.

Suggested `.gitignore` entries if they are not already present in the local project:

```gitignore
/target/
**/node_modules/
**/artifacts/
**/cache/
**/typechain-types/
**/coverage/
**/.env
**/.env.*
*.pem
*.key
```

Inspect ignored and untracked paths before every `git add`. In particular, verify no `.env`, RPC secrets, private keys, target directories, generated Solidity, transcripts containing sensitive information, or unrelated binary archives enter the commit.

After local verification, the owner may initialize Git and choose a hosting destination. **Do not create a remote, push, tag, release, or publish merely by running the grant-demo installer.**

Required hosted CI gates:

1. The `Rust / ubuntu-latest` job passes.
2. The `Rust / windows-latest` job passes.
3. The `CoFHE mock execution / Ubuntu` job passes.

Evidence must identify the exact commit associated with the passing results. Until these run on the selected remote, state "CI workflow prepared, hosted CI unverified".

The older V1 release gates are separate. Installing this patch into `fhec-v2` must not be interpreted as approval or publication of `fhec` V1.
