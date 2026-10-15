# Debian packaging policy and stable migration

Effective 2026-10-15, on both `main` and `dev`, every TwoDrive `.deb` distributed
as a release must be built through `dpkg-buildpackage` and debhelper using the
tracked `debian/` packaging. `debian/control` and `debian/changelog` are the
metadata authorities. Direct `dpkg-deb --build`, a second hand-written control
file, and copying prebuilt executables into a manually assembled release `.deb`
are not accepted release paths. `dpkg-deb` remains useful for inspecting packages.
This requirement applies to future Development `.deb` packages as well as stable
ones; it does not convert Windows or portable tar archives into Debian packages.

自 2026-10-15 起，`main` 与 `dev` 发布的所有 TwoDrive `.deb` 必须通过
`dpkg-buildpackage` 和 debhelper、使用版本控制中的 `debian/` 生成。
禁止用手工 `dpkg-deb --build` 或另一份 control 文件替代发布构建。
源码、二进制、调试符号及清单应一起保存并校验；不得把上游项目分发包称为
已被 Debian 官方仓库接收。Windows 和便携 tar 的格式不受此条改变。

## Build and release contract

1. Prepare immutable source inputs and the locked dependency closure before the
   Debian build. Required `debian/rules` targets must build and test offline.
   Declare compiler and native build dependencies. Do not silently bypass
   dependency checks, tests, Lintian failures, or missing output files.
2. Run `dpkg-buildpackage` with debhelper; retain `.dsc`, original/component
   tarballs, `.debian.tar.xz`, binary/debug packages, `.buildinfo` and `.changes`.
   Verify every referenced size and SHA-256 before releasing. Unsigned project
   artifacts are marked as such; a Debian archive upload needs proper signatures.
3. Check package dependencies, payload, ownership, conffiles, manuals, and
   licenses, plus Lintian. Exercise installation, upgrade from the prior release,
   removal/purge and installed application smoke tests in a disposable system.
4. Preserve account/config paths, tokens, SQLite schema and recovery state,
   content cache, pending uploads/deletes, CLI names, FUSE behavior and desktop
   entry points. Maintainer scripts must not read or migrate user home data,
   sign in to cloud accounts, clear recovery queues, or start/enable user mounts.
   Restart an existing production service only during a separately authorized
   local upgrade after checking active writes and creating a private backup.
5. Keep policy and packaging CI on both GitHub branches. `main` releases use
   stable application sources; Development features must not enter a stable
   release as a side effect of packaging work.

## Stable 0.2.14-2

The production Rust sources, Cargo.lock, Python desktop helper logic, service and
.desktop files are unchanged from stable `main` 7eb7f3d. This is a Debian
packaging revision, not a sync-engine feature release; the CLI still reports
0.2.14. debhelper may normalize Python shebangs to /usr/bin/python3. The native SQLite dependency is linked from the distribution and checked
with the existing database/recovery tests. Generated ELF/Python dependencies
replace the incomplete hand-written runtime list. User service enablement remains
opt-in; installing/removing the package does not stop or start user mounts.

The upstream preparation helper supplies a separate `orig-vendor` component,
including the locked Rust sources and their original licenses. Debian compilation
uses an empty Cargo home inside the source tree and requires no network/cache.
Third-party attribution/license files are also collected in the binary docs.
This retains the locked upstream dependency versions; it does not claim a full
Debian Rust dependency/copyright audit or acceptance into the Debian archive.
Official inclusion additionally needs distribution-packaged Rust dependencies,
review of remaining native convenience copies, licensing review, sponsorship,
and clean Debian distribution build/autopkgtest/piuparts validation.

See [build instructions](../debian/README.source) and
[incident and verification record](incidents.md#td-20261015-11stable-main-debian-打包流程与用户安装兼容性迁移).
Local candidate validation completed: 171 default Rust tests, 25 Nautilus,
4 Settings and 6 release-guard tests; all seven isolated FUSE regressions and
packaged daemon hydration/write/upload/move/delete/pin/release smoke passed.
An Ubuntu container upgraded the actual old release to the new package, then
removed/reinstalled/purged it while retaining every synthetic user-state hash.
The dependency checker fails on the old package and passes on the new package.
The first vendor-clean failure and subsequent CI timing-test failure are recorded
separately in the incident log. Full source/binary Lintian and final CI remain
pending at this snapshot.

GitHub dev policy PR #25 is merged as 4942f2ded44cdb805c397c90b932ca06c23dae0e.
Active repository ruleset 25148176 requires `Debian packaging policy` on both
main and dev; main's existing native-check and pull-request protection remains.
Final release and laptop validation will be appended after execution.
