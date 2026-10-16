#!/usr/bin/env python3
"""Build the shared Rust JNI adapter with the existing Rust/Android toolchains."""
import argparse
from contextlib import contextmanager
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time


@contextmanager
def build_lock(path):
    # Cargo protects compilation, but its lock ends before the artifact copy.
    # Keep configurations from replacing the shared output in that interval.
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+b') as handle:
        if os.name == 'nt':
            import msvcrt
            if handle.tell() == 0:
                handle.write(b'\0')
                handle.flush()
            handle.seek(0)
            msvcrt.locking(handle.fileno(), msvcrt.LK_LOCK, 1)
        else:
            import fcntl
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            if os.name == 'nt':
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', action='store_true', help='Build the JVM verification library')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--account-origin', default='')
    parser.add_argument('--auth-scheme', default='govorun')
    parser.add_argument('--styles-origin', default='https://styles.govorun.club')
    args = parser.parse_args()
    clients = Path(__file__).resolve().parents[2]
    cargo = shutil.which('cargo') or str(Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo')) / 'bin' / ('cargo.exe' if os.name == 'nt' else 'cargo'))
    if not Path(cargo).is_file():
        parser.error('Rust is required; configure cargo before building the shared core')
    env = os.environ.copy()
    # Trust is selected by this Gradle variant, never inherited accidentally
    # from a previous desktop/test build in the same terminal.
    env.pop('GOVORUN_ACCOUNT_ORIGIN', None)
    env.pop('GOVORUN_AUTH_SCHEME', None)
    env.pop('GOVORUN_STYLES_ORIGIN', None)
    if args.styles_origin: env['GOVORUN_STYLES_ORIGIN'] = args.styles_origin
    env['GOVORUN_APP_VERSION'] = re.search(r'versionName = \"([^\"]+)\"', (clients / 'android/app/build.gradle.kts').read_text()).group(1)
    if args.account_origin and args.account_origin.rstrip('/') != 'https://accounts.govorun.club':
        env['GOVORUN_ACCOUNT_ORIGIN'] = args.account_origin
        env['GOVORUN_AUTH_SCHEME'] = args.auth_scheme
    # All clients reuse one existing application cache; do not add an Android Rust cache.
    target_dir = Path(env.get('CARGO_TARGET_DIR', clients / 'desktop/target')).resolve()
    env['CARGO_TARGET_DIR'] = str(target_dir)
    command = [cargo, 'build', '--locked', '--release', '--manifest-path', str(clients / 'shared/Cargo.toml'), '-p', 'govorun-jvm-bridge']
    if args.host:
        command.extend(['--features', 'private-fixtures'])
        name = {'Darwin': 'libgovorun_jvm_bridge.dylib', 'Linux': 'libgovorun_jvm_bridge.so', 'Windows': 'govorun_jvm_bridge.dll'}[platform.system()]
        source = target_dir / 'release' / name
        destination = args.output / name
    else:
        sdk = os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT')
        if not sdk:
            parser.error('Set ANDROID_HOME to the existing Android SDK')
        host_tag = {'Darwin': 'darwin-x86_64', 'Linux': 'linux-x86_64', 'Windows': 'windows-x86_64'}[platform.system()]
        tools = Path(sdk) / 'ndk/28.2.13676358/toolchains/llvm/prebuilt' / host_tag / 'bin'
        linker = tools / ('aarch64-linux-android31-clang.cmd' if os.name == 'nt' else 'aarch64-linux-android31-clang')
        if not linker.is_file():
            parser.error('Install Android NDK 28.2.13676358 in the existing SDK')
        env['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER'] = str(linker)
        env['CC_aarch64_linux_android'] = str(linker)
        env['AR_aarch64_linux_android'] = str(tools / ('llvm-ar.exe' if os.name == 'nt' else 'llvm-ar'))
        env['CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS'] = '-C link-arg=-Wl,-z,max-page-size=16384'
        command.extend(['--target', 'aarch64-linux-android'])
        source = target_dir / 'aarch64-linux-android/release/libgovorun_jvm_bridge.so'
        destination = args.output / 'arm64-v8a/libgovorun_jvm_bridge.so'
    args.output.mkdir(parents=True, exist_ok=True)
    log = args.output / 'shared-core-build.log'
    started = time.monotonic()
    with build_lock(target_dir / '.govorun-jni-build.lock'):
        with log.open('w') as stream:
            result = subprocess.run(command, cwd=clients, env=env, stdout=stream, stderr=subprocess.STDOUT)
        if result.returncode:
            print(f'FAIL shared JNI build; {log}')
            print('\n'.join(log.read_text().splitlines()[-20:]))
            return result.returncode
        if not source.is_file() or source.is_symlink():
            raise RuntimeError('Shared JNI build produced no regular library')
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    print(f'PASS shared JNI {"JVM" if args.host else "Android arm64"}: {time.monotonic() - started:.2f}s; {destination}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
