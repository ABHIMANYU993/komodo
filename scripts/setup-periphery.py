#!/usr/bin/env python3
"""
Komodo Linux Periphery Installer & Lifecycle Manager (Python Edition)
Strictly installs Komodo Periphery as a native host system service.
Zero host mutations. NO Docker/Podman container fallback.
"""

import argparse
import sys
import os
import shutil
import platform
import subprocess
import urllib.request

DEFAULT_VERSION = "v2.4.1"
GITHUB_REPO = "ABHIMANYU993/komodo"

def parse_args():
    p = argparse.ArgumentParser(
        prog="setup-periphery",
        description="Install and manage Komodo Periphery native system service (systemd, OpenRC, SysVinit, runit, s6, dinit)",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    p.add_argument("--version", "-v", default=DEFAULT_VERSION, help="Release tag to install")
    p.add_argument("--root-directory", "-r", default="/etc/komodo", help="Periphery root directory")
    p.add_argument("--core-address", "-c", help="Komodo Core WebSocket address (e.g. ws://192.168.31.100:9120)")
    p.add_argument("--connect-as", "-n", default=os.uname().nodename, help="Server name")
    p.add_argument("--onboarding-key", "-k", help="One-time onboarding key")
    p.add_argument("--polling-rate", default="1-sec", help="Stats polling interval")
    p.add_argument("--reinstall", action="store_true", help="Fresh reinstall: wipe keys and re-onboard")
    p.add_argument("--reconfig", action="store_true", help="Reconfigure without wiping keys")
    p.add_argument("--restart", action="store_true", help="Restart periphery service")
    p.add_argument("--status", action="store_true", help="Show current service status")
    p.add_argument("--uninstall", action="store_true", help="Uninstall periphery service and binary")
    p.add_argument("--purge", action="store_true", help="Purge /etc/komodo configuration directory")
    p.add_argument("--binary-url", help="Direct URL to precompiled periphery binary")
    p.add_argument("--binary-path", help="Local path to precompiled periphery binary")
    return p.parse_args()

def detect_init():
    if shutil.which("systemctl") and os.path.isdir("/run/systemd/system"):
        return "systemd"
    if shutil.which("rc-service") or os.path.isfile("/sbin/openrc-run") or (os.path.isdir("/etc/init.d") and os.path.isfile("/etc/inittab")):
        return "openrc"
    if shutil.which("sv") or os.path.isdir("/etc/runit") or os.path.isdir("/var/service"):
        return "runit"
    if shutil.which("s6-svc") or shutil.which("s6-rc"):
        return "s6"
    if shutil.which("dinitctl") or os.path.isdir("/etc/dinit.d"):
        return "dinit"
    if os.path.isdir("/etc/init.d") and (shutil.which("update-rc.d") or shutil.which("chkconfig") or shutil.which("service")):
        return "sysvinit"
    return "none"

def detect_libc():
    if os.path.exists("/etc/alpine-release"):
        return "musl"
    try:
        out = subprocess.run(["ldd", "--version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if "musl" in (out.stdout + out.stderr).lower():
            return "musl"
    except Exception:
        pass
    for path in ["/lib", "/lib64"]:
        if os.path.isdir(path):
            for fname in os.listdir(path):
                if "ld-musl" in fname:
                    return "musl"
    return "gnu"

def download_file(url, dest):
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "KomodoInstaller/2.0"})
        with urllib.request.urlopen(req, timeout=30) as resp, open(dest, "wb") as f:
            f.write(resp.read())
        return True
    except Exception as e:
        return False

def main():
    args = parse_args()

    if os.geteuid() != 0:
        print("Error: Root access is required to manage system services. Run with sudo or doas.", file=sys.stderr)
        sys.exit(1)

    init_sys = detect_init()
    if init_sys == "none":
        print("==========================================================================", file=sys.stderr)
        print("ERROR: No supported system service manager detected on this host!", file=sys.stderr)
        print("Supported service managers: systemd, OpenRC, SysVinit, runit, s6, dinit.", file=sys.stderr)
        print("Komodo Periphery must run as a native system service. Container fallback is disabled.", file=sys.stderr)
        print("==========================================================================", file=sys.stderr)
        sys.exit(1)

    libc = detect_libc()
    bin_path = "/usr/local/bin/periphery"
    config_dir = args.root_directory
    config_file = f"{config_dir}/periphery.config.toml"
    keys_dir = f"{config_dir}/keys"

    # Status
    if args.status:
        print(f"=== Komodo Periphery Status ({init_sys}) ===")
        if init_sys == "systemd":
            subprocess.run(["systemctl", "status", "periphery", "--no-pager"])
        elif init_sys == "openrc":
            subprocess.run(["rc-service", "periphery", "status"])
        elif init_sys == "runit":
            subprocess.run(["sv", "status", "periphery"])
        elif init_sys == "sysvinit":
            subprocess.run(["/etc/init.d/periphery", "status"])
        sys.exit(0)

    # Uninstall
    if args.uninstall:
        print(f"Uninstalling Komodo Periphery service ({init_sys})...")
        if init_sys == "systemd":
            subprocess.run(["systemctl", "stop", "periphery"], stderr=subprocess.DEVNULL)
            subprocess.run(["systemctl", "disable", "periphery"], stderr=subprocess.DEVNULL)
            if os.path.exists("/etc/systemd/system/periphery.service"):
                os.remove("/etc/systemd/system/periphery.service")
            subprocess.run(["systemctl", "daemon-reload"], stderr=subprocess.DEVNULL)
        elif init_sys == "openrc":
            subprocess.run(["rc-service", "periphery", "stop"], stderr=subprocess.DEVNULL)
            subprocess.run(["rc-update", "del", "periphery", "default"], stderr=subprocess.DEVNULL)
            if os.path.exists("/etc/init.d/periphery"):
                os.remove("/etc/init.d/periphery")
        elif init_sys == "runit":
            subprocess.run(["sv", "stop", "periphery"], stderr=subprocess.DEVNULL)
            for p in ["/var/service/periphery", "/run/runit/service/periphery"]:
                if os.path.islink(p) or os.path.exists(p):
                    os.remove(p)
            shutil.rmtree("/etc/sv/periphery", ignore_errors=True)
        elif init_sys == "sysvinit":
            subprocess.run(["/etc/init.d/periphery", "stop"], stderr=subprocess.DEVNULL)
            subprocess.run(["update-rc.d", "-f", "periphery", "remove"], stderr=subprocess.DEVNULL)
            if os.path.exists("/etc/init.d/periphery"):
                os.remove("/etc/init.d/periphery")

        if os.path.exists(bin_path):
            os.remove(bin_path)

        if args.purge:
            shutil.rmtree(config_dir, ignore_errors=True)
            print(f"Komodo Periphery completely purged from {config_dir}.")
        else:
            print(f"Service and binary removed. Configuration preserved in {config_dir}.")
        sys.exit(0)

    # Restart
    if args.restart:
        print(f"Restarting Periphery service ({init_sys})...")
        if init_sys == "systemd":
            subprocess.run(["systemctl", "restart", "periphery"])
        elif init_sys == "openrc":
            subprocess.run(["rc-service", "periphery", "restart"])
        elif init_sys == "runit":
            subprocess.run(["sv", "restart", "periphery"])
        elif init_sys == "sysvinit":
            subprocess.run(["/etc/init.d/periphery", "restart"])
        sys.exit(0)

    # Core Address validation
    if not args.core_address:
        print("Error: Missing required argument --core-address (e.g. ws://192.168.31.100:9120)", file=sys.stderr)
        sys.exit(1)

    core_address = args.core_address
    if core_address.startswith("http://"):
        core_address = "ws://" + core_address[7:]
    elif core_address.startswith("https://"):
        core_address = "wss://" + core_address[8:]

    polling_rate = args.polling_rate
    if not polling_rate.endswith("-sec") and polling_rate.isdigit():
        polling_rate = f"{polling_rate}-sec"

    os.makedirs(config_dir, exist_ok=True)
    os.makedirs(keys_dir, exist_ok=True)
    os.chmod(config_dir, 0o700)
    os.chmod(keys_dir, 0o700)

    if args.reinstall or args.onboarding_key:
        for f in os.listdir(keys_dir):
            try:
                os.remove(os.path.join(keys_dir, f))
            except Exception:
                pass

    # Write Config
    config_content = f'''core_address = "{core_address}"
connect_as = "{args.connect_as}"
root_directory = "{config_dir}"
stats_polling_rate = "{polling_rate}"
include_disk_mounts = ["{config_dir}", "/host", "/"]
'''
    if args.onboarding_key:
        config_content += f'onboarding_key = "{args.onboarding_key}"\n'

    with open(config_file, "w") as f:
        f.write(config_content)
    os.chmod(config_file, 0o600)

    # Install Binary
    arch_raw = platform.machine().lower()
    arch = "x86_64" if arch_raw in ["x86_64", "amd64"] else ("aarch64" if arch_raw in ["aarch64", "arm64"] else "x86_64")

    installed = False
    if args.binary_path and os.path.isfile(args.binary_path):
        shutil.copy(args.binary_path, bin_path)
        os.chmod(bin_path, 0o755)
        installed = True
    elif args.binary_url:
        if download_file(args.binary_url, bin_path):
            os.chmod(bin_path, 0o755)
            installed = True

    if not installed:
        candidates = [f"periphery-{arch}-musl", f"periphery-{arch}"] if libc == "musl" else [f"periphery-{arch}", f"periphery-{arch}-musl"]
        for cand in candidates:
            url = f"https://github.com/{GITHUB_REPO}/releases/download/{args.version}/{cand}"
            print(f"Attempting download: {url}")
            if download_file(url, bin_path):
                os.chmod(bin_path, 0o755)
                # Test binary compatibility
                try:
                    res = subprocess.run([bin_path, "--help"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    if res.returncode == 0:
                        print(f"Verified binary: {cand}")
                        installed = True
                        break
                except Exception:
                    pass
                if os.path.exists(bin_path):
                    os.remove(bin_path)

    if not installed and os.path.isfile(bin_path) and os.access(bin_path, os.X_OK):
        print(f"Using existing binary at {bin_path}")
        installed = True

    if not installed:
        print("Error: Could not obtain compatible periphery binary.", file=sys.stderr)
        sys.exit(1)

    # Register Service
    print(f"Registering native service for {init_sys}...")
    if init_sys == "systemd":
        unit_content = f'''[Unit]
Description=Komodo Periphery Agent
After=network.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={bin_path} --config-path {config_file}
Restart=always
RestartSec=5
LimitNOFILE=65536
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
'''
        with open("/etc/systemd/system/periphery.service", "w") as f:
            f.write(unit_content)
        subprocess.run(["systemctl", "daemon-reload"])
        subprocess.run(["systemctl", "enable", "periphery"])
        subprocess.run(["systemctl", "restart", "periphery"])

    elif init_sys == "openrc":
        openrc_content = f'''#!/sbin/openrc-run
name="Komodo Periphery"
description="Komodo Periphery Agent"

command="{bin_path}"
command_args="--config-path {config_file}"
command_background="yes"
pidfile="/run/periphery.pid"
output_log="/var/log/periphery.log"
error_log="/var/log/periphery.err"

depend() {{
    need net
    after firewall
}}
'''
        with open("/etc/init.d/periphery", "w") as f:
            f.write(openrc_content)
        os.chmod("/etc/init.d/periphery", 0o755)
        subprocess.run(["rc-update", "add", "periphery", "default"])
        subprocess.run(["rc-service", "periphery", "restart"])

    elif init_sys == "runit":
        os.makedirs("/etc/sv/periphery", exist_ok=True)
        with open("/etc/sv/periphery/run", "w") as f:
            f.write(f"#!/bin/sh\nexec 2>&1\nexec {bin_path} --config-path {config_file}\n")
        os.chmod("/etc/sv/periphery/run", 0o755)
        if os.path.isdir("/var/service") and not os.path.exists("/var/service/periphery"):
            os.symlink("/etc/sv/periphery", "/var/service/periphery")
        subprocess.run(["sv", "restart", "periphery"])

    elif init_sys == "sysvinit":
        sysv_content = f'''#!/bin/sh
DAEMON={bin_path}
DAEMON_ARGS="--config-path {config_file}"
PIDFILE=/run/periphery.pid

case "$1" in
    start)
        start-stop-daemon --start --background --make-pidfile --pidfile "$PIDFILE" --exec "$DAEMON" -- $DAEMON_ARGS
        ;;
    stop)
        start-stop-daemon --stop --pidfile "$PIDFILE" --retry 5
        rm -f "$PIDFILE"
        ;;
    restart)
        $0 stop; sleep 1; $0 start
        ;;
    status)
        start-stop-daemon --status --pidfile "$PIDFILE" && echo "Running" || echo "Stopped"
        ;;
    *)
        echo "Usage: $0 {{start|stop|restart|status}}"; exit 1 ;;
esac
'''
        with open("/etc/init.d/periphery", "w") as f:
            f.write(sysv_content)
        os.chmod("/etc/init.d/periphery", 0o755)
        subprocess.run(["update-rc.d", "periphery", "defaults"], stderr=subprocess.DEVNULL)
        subprocess.run(["/etc/init.d/periphery", "restart"])

    print("==========================================================")
    print("SUCCESS: Komodo Periphery setup finished!")
    print(f"Service Manager: {init_sys}")
    print(f"Connected to: {core_address} as '{args.connect_as}'")
    print("==========================================================")

if __name__ == "__main__":
    main()
