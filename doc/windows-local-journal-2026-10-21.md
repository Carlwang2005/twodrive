# Windows private-cache journal bridge

This extends the unmerged PR #39 after the published alpha.2 source checkpoint
`7f0a77f8b969f0c90aaf3b65a3072b783ee705b3`. That tag and its assets stay unchanged.
It does **not** enable a production Windows sync root or writable GUI capability.

The smallest end-to-end contract uses a generated, versioned mock object and a
private cache: download by opaque ID, recheck the provider version after EOF,
fsync the complete cache, atomically publish its metadata and journal, persist a
new generation before returning a writable lease, fsync on close, claim a
conditional upload, and acknowledge only the exact claimed generation/version
and content. No protocol, database or CFAPI logic lives in Flutter.

## Ownership and persistence

`twodrive-core::CacheJournal` extends the existing `Database`, stable local IDs
and `FileState`; it does not introduce another synchronization state enum. Its
table is created only when this opt-in owner is opened. Existing FUSE databases
and metadata-preflight IPC do not open the owner or adopt foreign caches.

The engine owns one fixed Backend for the session. Arbitrary-string construction
is test-only; no real-provider production constructor is exposed. A domain-separated digest of
the existing provider/account binding and the canonical private cache root are
persisted before ownership recovery. Another identity/root is rejected before
advancing the epoch or resetting leases. The composition boundary must use the
existing profile-revision/provider binding, never a UI label or fresh arbitrary
namespace; changing accounts requires a different private state. Binding values
and paths are not exposed by status. No alternate Backend can be supplied to a
hydrate/upload/reconcile call after opening the bridge.

An OS-exclusive lifetime lock admits one private-cache owner. All read/write/
upload leases retain that owner. Directory handles on Windows deny ancestor
replacement; original ancestors and opened cache handles reject reparse/offline/
recall items. Only direct children of the private cache can be accessed. This is
not an importer or support for another client's placeholders.

The owner gate, SQLite IMMEDIATE transactions and real cache file locks serialize
intent and mutation. Durable readers/writers and generation counters are queried,
not caller-provided flags. A write intent is committed before a writable handle
is returned. Closing fsyncs before making it Dirty; it never implies upload.
Base identity, path, cache binding and opaque version must still match. Pending
deletes or parent metadata operations veto upload/release. Legacy cache-prune/
release APIs refuse journal-owned records, even with a stale Cached label.

The initial bridge serializes these checks per cache owner, including bounded-memory digest verification;
it does not hold that gate during network transfer. File locks and the persisted
upload claim remain live throughout transfer. More granular item scheduling is
a future performance change, not permission to weaken the checks.

## Acknowledgement and recovery

`experiments/twodrive-dev/src/local_files.rs` is the actual shared-engine bridge.
It uses the existing portable model and Provider contract on Windows and the
existing shared contract on Linux. Providers must explicitly advertise atomic
conditional upload. Metadata and object IDs remain distinct from paths.

An error other than a definitive version conflict can be a remotely accepted
write with a lost reply. It leaves the upload intent uncertain and blocks blind
retry, edits and release. Restart does not clear that intent. Reconciliation
downloads the provider object and rechecks identity/version/size after EOF;
only full matching content can acknowledge the upload. An unchanged remote
version permits a conditional retry; a changed, mismatching version becomes a
conflict with local bytes preserved. No automatic conflict overwrite is offered.

After the owner process exits, startup can reset old handle counts only after
acquiring the exclusive owner lock. A vanished writer becomes Dirty with
recovery_required; the cache must be reopened, locked and fsynced before upload.
An interrupted upload remains uncertain. Missing cache data or inconsistent
bindings/counters fail closed. Tests use subprocess exit without destructors;
they do not establish power-loss, OS reboot, physical Explorer or live-cloud
acceptance.

`engine_release_ready` is a diagnostic, never a detached authorization. The
actual clean-cache callback holds the owner gate, SQLite write lock and exclusive
file lock. The engine compares the complete file digest under that lock; an
untracked edit invalidates the generation and becomes Dirty. A future native
adapter must additionally own/recheck a CFAPI oplock, modified ranges, residency,
pin state and in-sync evidence at the action. No native dehydration callback is
connected by this slice. Resident/downloaded, pinned, pending, conflict/error and
acknowledged generations remain separate concepts.

## Validation

Windows lightweight harnesses compile the actual model/provider/bridge source;
the full desktop CI also runs its engine tests. Generated files exercise:

- Download → real edit/fsync → conditional upload → same-generation acknowledgement.
- Real readers and the owner lock block competing writers and clean-cache actions.
- Dirty, queued upload, stale Cached/generation/version/cache binding and pending
  delete/parent move remain release vetoes.
- Accepted upload with lost reply, offline before commit, full-byte reconciliation,
  remote conflict and provider-version changes during download.
- Actual subprocess exit during a write/upload, reopen, preserved complete bytes,
  and no blind recovery upload.

Local Windows results: 37 adapter/IPC/journal/actual-engine harness tests, the
existing 40 core regressions, format and strict Clippy passed. The journal
includes two real subprocess exit/reopen tests. Removing only the ownership
veto from the actual legacy release function makes the intended regression fail;
restoring byte-identical source makes it pass. This is a controlled source
mutation with generated data, not a test of a released writable client.
Exact-head full desktop/Linux/Windows CI is recorded in the PR after execution. Native
fixture results are separate: the explicitly approved hardened probe passed all
eight checks; its generated root was unregistered and all three files retained.
See [guarded evidence](windows-cfapi-guarded-validation-2026-10-21.json).

Two independent-review regressions failed against the previous implementation:
same-length editing between an earlier hash and final reconciliation could
acknowledge stale content, and a rejected second hydration could replace owned
cache metadata. Reconciliation now hashes under its final file lease through
commit; hydration checks ownership before metadata mutation and serializes its
claim with publication. Both deterministic regressions pass after these fixes.

Remaining integration: real native callback cancellation/hydration and journal
ownership, stable item-status IPC/UI, Explorer state/icon/thumbnail behavior,
native rename/delete and conflict-resolution/restart scenarios. These remain
capability-disabled. Real accounts, installed clients, existing sync roots,
vendor caches and peer UDP/QUIC code are untouched.
