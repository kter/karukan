ADDON_DIR = fcitx5-addon
BUILD_DIR = $(ADDON_DIR)/build
CARGO_MANIFEST = ../../Cargo.toml
DATA_DIR = $(HOME)/.local/share/karukan-im
DICT_URL = https://github.com/togatoga/karukan/releases/latest/download/dict.tgz
ENV_FILE = $(HOME)/.config/environment.d/fcitx5-karukan.conf
LOG_FILE = $(HOME)/.cache/karukan-im/fcitx5.log

# System install by default. `install-user` re-enters with the user-local
# prefix and no sudo, so both paths share one build/install recipe.
PREFIX ?= /usr
SUDO ?= sudo
# Extra cmake flags, e.g. `make build CMAKE_FLAGS=-DKARUKAN_NATIVE=OFF` for a
# binary that runs on machines other than the one that built it.
CMAKE_FLAGS ?=

.PHONY: all deps check-not-sudo build install install-user install-dict env-file restart test fmt lint clean

all: build

# Only the install step needs root. Under `sudo make` the cargo build runs as
# root too, where rustup has no default toolchain and the build dies with
# "could not choose a version of cargo to run". A plain root shell (no
# SUDO_USER) is left alone — that one is deliberate.
check-not-sudo:
	@if [ -n "$$SUDO_USER" ]; then \
		echo "Run this as your normal user: 'make $(MAKECMDGOALS)'."; \
		echo "The install step elevates itself; 'sudo make' builds as root and rustup fails."; \
		exit 1; \
	fi

# Debian/Ubuntu only. Other distros: see the package list in README.md.
deps:
	sudo apt install fcitx5 fcitx5-modules-dev libfcitx5core-dev \
		libfcitx5config-dev libfcitx5utils-dev extra-cmake-modules \
		cmake make gcc g++ pkg-config \
		clang libclang-dev \
		libssl-dev libxkbcommon-dev

# Configure and build the addon. The cmake project drives the cargo build of
# karukan-fcitx5, so there is no separate cargo step here.
build: check-not-sudo
	cmake -B $(BUILD_DIR) -S $(ADDON_DIR) -DCMAKE_INSTALL_PREFIX=$(PREFIX) $(CMAKE_FLAGS)
	cmake --build $(BUILD_DIR) -j

# System install into /usr. Needs sudo, but no FCITX_ADDON_DIRS setup.
install: build install-dict
	$(SUDO) cmake --install $(BUILD_DIR)
	@echo "Installed to $(PREFIX). Restart fcitx5 with: make restart"
	@echo "Then add Karukan in fcitx5-configtool (Available Input Method > karukan)."

# User-local install into ~/.local. No sudo, but fcitx5 only finds the addon
# once FCITX_ADDON_DIRS names both the local and the system directory —
# `make env-file` writes that, and it takes effect on the next login.
install-user:
	$(MAKE) install PREFIX=$(HOME)/.local SUDO=
	@echo "User-local install: set the addon path with 'make env-file', then log out and back in."

# Download the prebuilt system dictionary unless one is already installed.
# Non-fatal on failure (e.g. offline) — the IME works model-only without it.
install-dict:
	@if [ -f "$(DATA_DIR)/dict.bin" ]; then \
		echo "System dictionary already installed: $(DATA_DIR)/dict.bin (skipping download)"; \
	else \
		echo "Downloading system dictionary from $(DICT_URL) ..."; \
		tmp=$$(mktemp -d) && \
		( curl -fL --progress-bar -o "$$tmp/dict.tgz" "$(DICT_URL)" && \
		  tar xzf "$$tmp/dict.tgz" -C "$$tmp" && \
		  mkdir -p "$(DATA_DIR)" && \
		  cp "$$tmp/dict.bin" "$(DATA_DIR)/" && \
		  echo "Installed system dictionary to $(DATA_DIR)/dict.bin" \
		) || echo "WARNING: system dictionary install failed; install manually (see docs/dictionary.md)"; \
		rm -rf "$$tmp"; \
	fi

# Point fcitx5 at the user-local addon directory. Written to
# ~/.config/environment.d/ rather than a shell profile because fcitx5 starts
# with the login session. The system directory must stay in the list, or
# fcitx5 loses its own addons (wayland, classicui, ...).
env-file:
	@mkdir -p "$(dir $(ENV_FILE))"
	@system_dir="$$(pkg-config --variable=libdir Fcitx5Core)/fcitx5" && \
		echo "FCITX_ADDON_DIRS=$(HOME)/.local/lib/fcitx5:$$system_dir" > "$(ENV_FILE)" && \
		echo "Wrote $(ENV_FILE):" && cat "$(ENV_FILE)"
	@echo "Log out and back in for this to take effect."

# Stop the running fcitx5, wait for it to actually exit, then start a fresh
# daemon. `fcitx5 -r` starts the new instance and evicts the old one
# concurrently, so the two overlap and nothing here can tell whether the
# result is the instance we asked for; stopping first makes the order
# deterministic and lets the PID check below actually mean something.
#
# The log goes to $(LOG_FILE) rather than the terminal. Under `-d` fcitx5
# keeps the inherited stdout after make has exited, so its output arrives at
# a prompt that has already returned (and `make restart | ...` never
# finishes, because the daemon holds the pipe open). A file keeps the
# terminal usable and keeps the startup errors readable after the fact.
#
# NOTE: restarting fcitx5 does not restore input in clients that were already
# running -- X11/XWayland apps in particular hold their XIM connection to the
# dead instance. If typing stops working in a window that was open across the
# restart, restart that application.
restart:
	@mkdir -p "$(dir $(LOG_FILE))"
	@old=$$(pgrep -x fcitx5 || true); \
	if [ -n "$$old" ]; then \
		echo "Stopping fcitx5 (pid $$old) ..."; \
		kill $$old 2>/dev/null || true; \
		i=0; while pgrep -x fcitx5 >/dev/null && [ $$i -lt 50 ]; do sleep 0.1; i=$$((i+1)); done; \
		if pgrep -x fcitx5 >/dev/null; then \
			echo "Did not exit in 5s; sending SIGKILL."; \
			pkill -KILL -x fcitx5 || true; sleep 0.5; \
		fi; \
	fi; \
	fcitx5 -d >"$(LOG_FILE)" 2>&1; \
	i=0; while [ $$i -lt 50 ] && ! pgrep -x fcitx5 >/dev/null; do sleep 0.1; i=$$((i+1)); done; \
	new=$$(pgrep -x fcitx5 || true); \
	if [ -z "$$new" ]; then \
		echo "ERROR: fcitx5 did not start. Log: $(LOG_FILE)"; exit 1; \
	fi; \
	if [ "$$new" = "$$old" ]; then \
		echo "ERROR: fcitx5 (pid $$new) was never replaced. Log: $(LOG_FILE)"; exit 1; \
	fi; \
	echo "fcitx5 restarted (pid $$new). Log: $(LOG_FILE)"; \
	echo "Check that the addon came up with: grep karukan $(LOG_FILE)"

test:
	cargo test --manifest-path $(CARGO_MANIFEST) -p karukan-fcitx5 -p karukan-im

fmt:
	cargo fmt --manifest-path $(CARGO_MANIFEST) -p karukan-fcitx5

lint:
	cargo clippy --manifest-path $(CARGO_MANIFEST) -p karukan-fcitx5 --all-targets -- -D warnings

clean:
	rm -rf $(BUILD_DIR)
