# Windows Explorer integration — generated Provider only

This branch starts at dev `507fef1` and explicitly includes the unmerged
[PR #39](https://github.com/LumiaBlack51/twodrive/pull/39) dependency `7f98410`.
It extends that engine/journal/IPC implementation, without republishing alpha.3.
The production native mount, real-account native writes, rename/delete,
conflict resolution, persistent root restart and private-cache eviction remain
capability-disabled. This is not a completed Windows synchronization release.

## Explorer and engine ownership

The opt-in generated host registers its new guarded root with
`StorageProviderSyncRootManager`, alongside CFAPI. Each root has a unique
per-user registration ID. No existing root registration is enumerated or changed.
Shell registration is verified by both its ID and folder binding before removal.
There is no COM extension installation, global icon overlay or Explorer restart.

Windows derives availability and pin icons from the actual placeholders. A custom
Shell property separately reports acknowledged generations, pending recovery,
real read failures and active hydration. No successful download is relabelled as
an uploaded generation. Unknown state is explicit. The generated read-only
Provider cannot produce a cloud write conflict; no synthetic conflict is shown.
Root summary counts native resident files, private cache copies and pinned intent
separately; the worst observed child sync condition is preserved.

The owner observes only its bounded generated leaves every 500 ms. Native Shell
pin/free-space attribute changes request the same existing engine-controlled
read/dehydration operations. The OS cannot automatically dehydrate: the policy
does not enable AutoDehydrationAllowed, and external dehydration notifications
remain denied. A free-space intent still requires the exclusive CFAPI reference,
journal/file locks, acknowledged digest, generation, and clean/pin/handle checks.
Private cache eviction stays false.

Flutter negotiates the existing schema/scope and individual pin/offline-test/
callback-reconnect capabilities. It uses private IPC for all operations, and
opens only the engine-returned generated root through the existing file-manager
helper. Reconnect here means disconnect/connect of this live callback session;
it is not persistent process/OS restart recovery.

## Exit must preserve files

Microsoft documents that `CfUnregisterSyncRoot` **permanently deletes unhydrated
placeholders**. The earlier unconditional shutdown was unsafe for an untouched
fixture. Before explicit stop or normal owner destruction, this host retains its
deterministic generated data, without using a real account or network. The native
Session additionally refuses either Shell or CFAPI unregistration while a known
placeholder is incomplete or has changed logical length. A dirty incomplete
file is kept for recovery; shutdown reports incomplete cleanup instead of
claiming that unregistration succeeded. Generated files, journal and cache remain.
This fixture teardown policy is not a production sync-root uninstall policy.

## Acceptance matrix

| Scenario | Evidence / limit |
| --- | --- |
| Engine generation/digest/identity/open-handle/dirty guards | Existing actual-source regressions retained; final results recorded in PR |
| Failed read clears only after successful retry | Actual EngineSource regression |
| Error/pending/unknown folder aggregate vs separate residency/cache counts | Actual engine-source regression |
| Flutter pin/offline/reconnect capability gates and late-client isolation | Widget/client tests; final results recorded in PR |
| Shell registration/property metadata does not hydrate content | Explicit new-root ignored test; pending execution |
| OS Chinese/long-path reads, pin/unpin, release, offline retry and live reconnect | Explicit new-root ignored test; pending execution |
| Untouched placeholders survive normal exit | Separate explicit new-root ignored test; pending execution |
| Explorer cloud/check/pin/pending/error visuals and context-menu interaction | Unverified: native desktop control entry point unavailable in this session |
| Actual GUI → IPC → native root interaction | Unverified pending current-binary desktop acceptance |
| Explorer thumbnail/indexer/AV hydration behavior | Unverified; metadata-only API checks do not establish this |
| Persistent process/root reconnect, native write/rename/delete/conflict | Disabled/unimplemented |
| Existing OneDrive/Google Drive directories/cache | Read-only metadata preflight only; no adoption, client stop, credentials or private-cache access |

No ignored registration test runs automatically in CI. All actual native tests
must use separately approved new generated roots, retain files and verify both
Shell and CFAPI cleanup. Previous task roots and approvals are never reused.
Screenshots, when available, must identify the exact source SHA and Windows build;
API or handle state alone is not visual acceptance.

Primary references:
[Shell registration sample](https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/CloudMirror/CloudMirror/CloudProviderRegistrar.cpp),
[custom item properties](https://learn.microsoft.com/en-us/uwp/api/windows.storage.provider.storageprovideritemproperties.setasync),
[hydration and dehydration policy](https://learn.microsoft.com/en-us/windows/win32/api/cfapi/nf-cfapi-cfregistersyncroot),
[unregistration data semantics](https://learn.microsoft.com/en-us/windows/win32/api/cfapi/nf-cfapi-cfunregistersyncroot).
