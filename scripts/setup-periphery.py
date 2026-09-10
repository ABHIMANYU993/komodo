#!/usr/bin/env python3
import argparse
import sys
import os
import shutil
import platform
import subprocess
import json
import urllib.request

DEFAULT_VERSION = "2"
CONFIG_TEMPLATE_URL = "https://raw.githubusercontent.com/ABHIMANYU993/komodo/main/config/periphery.config.toml"
DOCKER_IMAGE = "ghcr.io/abhimanyu993/komodo-periphery:2"

def parse_args():
	p = argparse.ArgumentParser(
		prog="setup-periphery",
		description="Install and manage Komodo Periphery on systemd, OpenRC, or Docker",
		formatter_class=argparse.ArgumentDefaultsHelpFormatter,
	)

	p.add_argument(
		"--version", "-v",
		default=DEFAULT_VERSION,
		help="Install a specific Komodo version/tag"
	)

	p.add_argument(
		"--user", "-u",
		action="store_true",
		help="Install systemd '--user' service (systemd only)"
	)

	p.add_argument(
		"--root-directory", "-r",
		default="/etc/komodo",
		help="Specify a specific Periphery root directory."
	)

	p.add_argument(
		"--core-address", "-c",
		help="Specify the Komodo Core address (e.g. ws://192.168.31.100:9120)."
	)

	p.add_argument(
		"--connect-as", "-n",
		default=os.uname().nodename,
		help="Specify the Server name to connect as. Defaults to hostname."
	)

	p.add_argument(
		"--onboarding-key", "-k",
		help="Give an onboarding key for automatic Server onboarding into Komodo Core."
	)

	p.add_argument(
		"--polling-rate",
		default="1-sec",
		help="Stats polling interval (1-sec, 2-sec, 5-sec, etc.)"
	)

	p.add_argument(
		"--container",
		action="store_true",
		help="Force installation via Docker/Podman container"
	)

	p.add_argument(
		"--reinstall",
		action="store_true",
		help="Reset keys and re-onboard freshly"
	)

	p.add_argument(
		"--reconfig",
		action="store_true",
		help="Reconfigure core address and node name without reinstalling"
	)

	p.add_argument(
		"--uninstall",
		action="store_true",
		help="Uninstall Periphery and service"
	)

	p.add_argument(
		"--status",
		action="store_true",
		help="Show current running status"
	)

	p.add_argument(
		"--force-service-file",
		action="store_true",
		help="Recreate the service file even if it already exists."
	)

	p.add_argument(
		"--config-url",
		default=CONFIG_TEMPLATE_URL,
		help="Use a custom config url."
	)

	p.add_argument(
		"--binary-url",
		default="https://github.com/moghtech/komodo/releases/download",
		help="Use alternate binary source"
	)

	p.add_argument(
		"--core-public-keys",
		default="",
		help="Trust Komodo Core public keys. Comma separated list of base58 encoded public keys."
	)

	return p.parse_args()

def detect_init():
	if shutil.which("systemctl") is not None and os.path.exists("/run/systemd/system/"):
		return "systemd"
	if shutil.which("rc-service") is not None or os.path.exists("/sbin/openrc-run"):
		return "openrc"
	if shutil.which("docker") is not None:
		return "docker"
	if shutil.which("podman") is not None:
		return "podman"
	return "generic"

def is_alpine():
	return os.path.exists("/etc/alpine-release")

def load_paths(args, init_sys):
	home_dir = os.environ.get('HOME', '/root')
	if args.user and init_sys == "systemd":
		return [
			home_dir,
			f'{home_dir}/.local/bin',
			f'{home_dir}/.config/komodo',
			f'{home_dir}/.config/systemd/user',
		]
	else:
		service_dir = "/etc/systemd/system" if init_sys == "systemd" else "/etc/init.d"
		return [
			home_dir,
			"/usr/local/bin",
			"/etc/komodo",
			service_dir,
		]

def map_config_line(args, home_dir, line):
	if line.startswith("root_directory ="):
		if args.root_directory is not None:
			return f'root_directory = "{args.root_directory}"'
		if args.user:
			return f'root_directory = "{home_dir}/komodo"'
	if line.startswith("# core_address =") and args.core_address is not None:
		return f'core_address = "{args.core_address}"'
	if line.startswith("# connect_as ="):
		return f'connect_as = "{args.connect_as}"'
	if line.startswith("# onboarding_key =") and args.onboarding_key is not None:
		return f'onboarding_key = "{args.onboarding_key}"'
	if line.startswith("# core_public_keys =") and args.core_public_keys:
		return f'core_public_keys = "{args.core_public_keys}"'
	if line.startswith("stats_polling_rate ="):
		return f'stats_polling_rate = "{args.polling_rate}"'
	return line

def write_config(args, home_dir, config_dir):
	config_file = f'{config_dir}/periphery.config.toml'
	if os.path.isfile(config_file) and not args.reinstall and not args.reconfig:
		print(f'Config at {config_file} already exists, updating if needed...')
	
	print(f'Writing config at {config_file}')
	if not os.path.isdir(config_dir):
		os.makedirs(config_dir, exist_ok=True)

	try:
		req = urllib.request.Request(args.config_url, headers={'User-Agent': 'KomodoInstaller/2.0'})
		template = urllib.request.urlopen(req, timeout=10).read().decode("utf-8").split("\n")
		lines = [map_config_line(args, home_dir, line) for line in template]
		config = "\n".join(lines)
	except Exception:
		# Fallback static config
		config = f'''# Komodo Periphery Configuration
core_address = "{args.core_address or ''}"
connect_as = "{args.connect_as}"
root_directory = "{config_dir}"
stats_polling_rate = "{args.polling_rate}"
'''
		if args.onboarding_key:
			config += f'onboarding_key = "{args.onboarding_key}"\n'

	with open(config_file, "w", encoding="utf-8", newline="\n") as f:
		f.write(config)
	os.chmod(config_file, 0o600)

def install_openrc_service(args, config_dir, use_docker=False):
	service_file = "/etc/init.d/periphery"
	print(f"Creating OpenRC service at {service_file}...")
	runtime = "podman" if shutil.which("podman") and not shutil.which("docker") else "docker"
	keys_dir = f"{config_dir}/keys"
	onboard_env = f'-e PERIPHERY_ONBOARDING_KEY="{args.onboarding_key}" \\' if args.onboarding_key else ''
	
	if use_docker:
		content = f'''#!/sbin/openrc-run
name="Komodo Periphery (Container)"
description="Komodo Periphery Agent running in {runtime}"

depend() {{
	need net {runtime}
}}

start() {{
	ebegin "Starting Komodo Periphery container"
	{runtime} start komodo-periphery 2>/dev/null || {runtime} run -d \\
		--name komodo-periphery \\
		--network host \\
		--restart unless-stopped \\
		-e PERIPHERY_CORE_ADDRESS="{args.core_address}" \\
		-e PERIPHERY_CONNECT_AS="{args.connect_as}" \\
		{onboard_env}
		-e PERIPHERY_STATS_POLLING_RATE="{args.polling_rate}" \\
		-e PERIPHERY_INCLUDE_DISK_MOUNTS="{config_dir},/host,/" \\
		-v /var/run/docker.sock:/var/run/docker.sock:ro \\
		-v /proc:/proc:ro \\
		-v {keys_dir}:/config/keys \\
		-v {config_dir}:{config_dir} \\
		{DOCKER_IMAGE}
	eend $?
}}

stop() {{
	ebegin "Stopping Komodo Periphery container"
	{runtime} stop komodo-periphery
	eend $?
}}
'''
	else:
		content = f'''#!/sbin/openrc-run
name="Komodo Periphery"
description="Agent to connect with Komodo Core"
command="/usr/local/bin/periphery"
command_args="--config-path {config_dir}/periphery.config.toml"
command_background="yes"
pidfile="/run/periphery.pid"

depend() {{
	need net
	after firewall
}}
'''
	with open(service_file, "w", encoding="utf-8", newline="\n") as f:
		f.write(content)
	os.chmod(service_file, 0o755)

	os.system("rc-update add periphery default 2>/dev/null || true")
	os.system("rc-service periphery restart 2>/dev/null || rc-service periphery start 2>/dev/null || true")
	print("Periphery OpenRC service registered and started.")

def install_systemd_service(args, home_dir, bin_dir, config_dir, service_dir):
	service_file = f'{service_dir}/periphery.service'
	if not os.path.isdir(service_dir):
		os.makedirs(service_dir, exist_ok=True)

	print(f'Creating systemd service at {service_file}')
	content = (
		"[Unit]\n"
		"Description=Agent to connect with Komodo Core\n"
		"After=network.target\n"
		"\n"
		"[Service]\n"
		f'Environment="HOME={home_dir}"\n'
		f'ExecStart=/bin/sh -lc "{bin_dir}/periphery --config-path {config_dir}/periphery.config.toml"\n'
		"Restart=always\n"
		"RestartSec=5\n"
		"TimeoutStartSec=0\n"
		"\n"
		"[Install]\n"
		"WantedBy=default.target\n"
	)
	with open(service_file, "w", encoding="utf-8", newline="\n") as f:
		f.write(content)

	user = " --user" if args.user else ""
	os.system(f'systemctl{user} daemon-reload')
	os.system(f'systemctl{user} enable periphery 2>/dev/null || true')
	os.system(f'systemctl{user} restart periphery 2>/dev/null || systemctl{user} start periphery 2>/dev/null || true')
	print("Periphery systemd service registered and started.")

def install_docker_direct(args, config_dir):
	runtime = "podman" if shutil.which("podman") else "docker"
	print(f"Deploying Periphery container via {runtime}...")
	os.system(f"{runtime} rm -f komodo-periphery 2>/dev/null || true")
	
	keys_dir = f"{config_dir}/keys"
	onboard_env = f'-e PERIPHERY_ONBOARDING_KEY="{args.onboarding_key}" \\' if args.onboarding_key else ""
	sock = "/run/podman/podman.sock" if runtime == "podman" and os.path.exists("/run/podman/podman.sock") else "/var/run/docker.sock"
	
	cmd = f'''{runtime} run -d \\
		--name komodo-periphery \\
		--network host \\
		--restart unless-stopped \\
		-e PERIPHERY_CORE_ADDRESS="{args.core_address}" \\
		-e PERIPHERY_CONNECT_AS="{args.connect_as}" \\
		{onboard_env}
		-e PERIPHERY_STATS_POLLING_RATE="{args.polling_rate}" \\
		-e PERIPHERY_INCLUDE_DISK_MOUNTS="{config_dir},/host,/" \\
		-v {sock}:/var/run/docker.sock:ro \\
		-v /proc:/proc:ro \\
		-v {keys_dir}:/config/keys \\
		-v {config_dir}:{config_dir} \\
		{DOCKER_IMAGE}'''
	
	res = os.system(cmd)
	if res == 0:
		print(f"Periphery container deployed and running via {runtime}.")
	else:
		print(f"Failed to launch Periphery container with {runtime}.")

def do_uninstall(args, init_sys, config_dir, service_dir):
	print("Uninstalling Komodo Periphery...")
	if init_sys == "systemd":
		user = " --user" if args.user else ""
		os.system(f"systemctl{user} stop periphery 2>/dev/null || true")
		os.system(f"systemctl{user} disable periphery 2>/dev/null || true")
		svc = f"{service_dir}/periphery.service"
		if os.path.exists(svc):
			os.remove(svc)
		os.system(f"systemctl{user} daemon-reload 2>/dev/null || true")
	elif init_sys == "openrc":
		os.system("rc-service periphery stop 2>/dev/null || true")
		os.system("rc-update del periphery default 2>/dev/null || true")
		if os.path.exists("/etc/init.d/periphery"):
			os.remove("/etc/init.d/periphery")
	
	if shutil.which("docker"):
		os.system("docker rm -f komodo-periphery 2>/dev/null || true")
	if shutil.which("podman"):
		os.system("podman rm -f komodo-periphery 2>/dev/null || true")
	
	bin_path = "/usr/local/bin/periphery"
	if os.path.exists(bin_path):
		os.remove(bin_path)

	print("Komodo Periphery uninstalled successfully.")
	sys.exit(0)

def main():
	args = parse_args()
	init_sys = detect_init()

	print("=========================================")
	print("       KOMODO PERIPHERY INSTALLER        ")
	print(f" Detected Init/Runtime: {init_sys.upper()} ")
	print("=========================================")

	[home_dir, bin_dir, config_dir, service_dir] = load_paths(args, init_sys)

	if args.uninstall:
		do_uninstall(args, init_sys, config_dir, service_dir)

	if args.status:
		print("=== Service Status ===")
		if init_sys == "systemd":
			user = " --user" if args.user else ""
			os.system(f"systemctl{user} status periphery")
		elif init_sys == "openrc":
			os.system("rc-service periphery status")
		if shutil.which("docker"):
			os.system("docker ps -f name=komodo-periphery")
		if shutil.which("podman"):
			os.system("podman ps -f name=komodo-periphery")
		sys.exit(0)

	if not args.core_address:
		print("Error: --core-address is required (e.g. ws://192.168.31.100:9120)")
		sys.exit(1)

	if args.reinstall:
		keys_dir = f"{config_dir}/keys"
		if os.path.isdir(keys_dir):
			print(f"Purging keys in {keys_dir} for fresh reinstall...")
			shutil.rmtree(keys_dir, ignore_errors=True)

	write_config(args, home_dir, config_dir)

	# Alpine Linux / musl or Container requested
	if args.container or is_alpine() or init_sys not in ["systemd"]:
		if shutil.which("docker") or shutil.which("podman"):
			if init_sys == "openrc":
				install_openrc_service(args, config_dir, use_docker=True)
			else:
				install_docker_direct(args, config_dir)
		else:
			if is_alpine():
				print("Error: Alpine Linux uses musl libc and requires Docker or Podman to run the Periphery agent.")
				print("Please ensure Docker or Podman is installed and running on the host.")
				sys.exit(1)
			else:
				print("Error: Unsupported init system and neither Docker nor Podman was found.")
				sys.exit(1)
	else:
		# Standard systemd native installation
		# Stop existing instance if running
		user = " --user" if args.user else ""
		os.system(f'systemctl{user} stop periphery 2>/dev/null || true')

		arch = platform.machine().lower()
		periphery_bin = "periphery-aarch64" if arch in ["aarch64", "arm64"] else "periphery-x86_64"
		bin_path = f"{bin_dir}/periphery"
		os.makedirs(bin_dir, exist_ok=True)

		download_url = f"{args.binary_url}/v1.17.2/{periphery_bin}"
		print(f"Downloading Periphery binary for {arch} from {download_url}...")
		dl_res = os.system(f"curl -fsSL {download_url} -o {bin_path} || wget -qO {bin_path} {download_url}")
		if dl_res != 0 or not os.path.exists(bin_path) or os.path.getsize(bin_path) == 0:
			# Fallback to docker container if binary download fails
			if shutil.which("docker") or shutil.which("podman"):
				print("Native binary download failed. Falling back to container runtime...")
				install_docker_direct(args, config_dir)
				sys.exit(0)
			print("Error: Failed to download periphery binary.")
			sys.exit(1)
		
		os.chmod(bin_path, 0o755)
		install_systemd_service(args, home_dir, bin_dir, config_dir, service_dir)

	print("\nSetup finished successfully!")
	print(f"Node '{args.connect_as}' configured to connect to '{args.core_address}'.")

if __name__ == "__main__":
	main()
