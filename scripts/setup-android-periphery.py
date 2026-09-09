#!/usr/bin/env python3
"""
Komodo Android Periphery Installer (Python Compatibility Wrapper)

NOTE: This Python script is an optional convenience wrapper.
The CANONICAL installer for Android devices is `setup-android-periphery.sh`.
Android devices MUST NOT require Python, Rust, Cargo, or ADB to install Komodo Periphery.
"""

import argparse
import os
import subprocess
import sys

def parse_args():
    parser = argparse.ArgumentParser(
        description="Komodo Android Periphery Installer (Python Compatibility Wrapper)",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "--core-address", "-c",
        required=True,
        help="Specify the Komodo Core address (e.g. ws://192.168.31.80:9120)",
    )

    parser.add_argument(
        "--connect-as", "-n",
        default=None,
        help="Specify the Server name to connect as. Defaults to hostname.",
    )

    parser.add_argument(
        "--onboarding-key", "-k",
        default=None,
        help="One-time onboarding token for initial registration into Komodo Core.",
    )

    parser.add_argument(
        "--version", "-v",
        default="latest",
        help="Install a specific version, like 'v0.1.0' or 'latest'",
    )

    parser.add_argument(
        "--artifact-url",
        default=None,
        help="Direct URL to komodo-android-periphery.zip artifact",
    )

    parser.add_argument(
        "--artifact-file",
        default=None,
        help="Local file path to prebuilt komodo-android-periphery.zip",
    )

    parser.add_argument(
        "--non-interactive",
        action="store_true",
        help="Run non-interactively without prompting for reboot",
    )

    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable verbose output",
    )

    parser.add_argument(
        "--force",
        action="store_true",
        help="Force reinstallation even if already installed and healthy",
    )

    return parser.parse_args()

def main():
    args = parse_args()
    script_dir = os.path.dirname(os.path.abspath(__file__))
    sh_installer = os.path.join(script_dir, "setup-android-periphery.sh")

    if not os.path.exists(sh_installer):
        print(f"Error: Canonical shell installer not found at {sh_installer}", file=sys.stderr)
        sys.exit(1)

    cmd = ["/system/bin/sh" if os.path.exists("/system/bin/sh") else "sh", sh_installer]
    cmd.append(f"--core-address={args.core_address}")
    
    if args.connect_as:
        cmd.append(f"--connect-as={args.connect_as}")
    if args.onboarding_key:
        cmd.append(f"--onboarding-key={args.onboarding_key}")
    if args.version:
        cmd.append(f"--version={args.version}")
    if args.artifact_url:
        cmd.append(f"--artifact-url={args.artifact_url}")
    if args.artifact_file:
        cmd.append(f"--artifact-file={args.artifact_file}")
    if args.non_interactive:
        cmd.append("--non-interactive")
    if args.verbose:
        cmd.append("--verbose")
    if args.force:
        cmd.append("--force")

    try:
        res = subprocess.run(cmd)
        sys.exit(res.returncode)
    except Exception as e:
        print(f"Failed to execute installer: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
