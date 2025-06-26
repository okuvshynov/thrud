.PHONY: build install uninstall start stop restart status logs clean help

# Development installation paths
DEV_BIN_DIR := $(HOME)/.local/bin
DEV_SERVICE_DIR := $(HOME)/Library/LaunchAgents
DEV_LOG_DIR := $(HOME)/.thrud/logs
DEV_PLIST := com.thrud.collector.dev.plist

help:
	@echo "Thrud Development Commands:"
	@echo "  build         - Build release binaries"
	@echo "  install       - Install binaries to ~/.local/bin"
	@echo "  install-service - Install and configure launch agent"
	@echo "  start         - Start the collector service"
	@echo "  stop          - Stop the collector service"
	@echo "  restart       - Restart the collector service"
	@echo "  status        - Show service and database status"
	@echo "  logs          - Follow collector logs"
	@echo "  clean         - Remove installed files and stop service"
	@echo "  uninstall     - Complete cleanup including database"

build:
	@echo "🔨 Building Thrud..."
	cargo build --release
	@echo "✅ Build complete"

install: build
	@echo "📦 Installing Thrud binaries..."
	mkdir -p $(DEV_BIN_DIR)
	mkdir -p $(DEV_LOG_DIR)
	cp target/release/thrud-collector $(DEV_BIN_DIR)/
	cp target/release/thrud-demo $(DEV_BIN_DIR)/
	cp target/release/thrud-chart-query $(DEV_BIN_DIR)/
	@echo "✅ Binaries installed to $(DEV_BIN_DIR)"
	@echo "💡 Add $(DEV_BIN_DIR) to your PATH if not already added"

install-service: install
	@echo "⚙️  Installing launch agent..."
	mkdir -p $(DEV_SERVICE_DIR)
	sed -e 's|{{BIN_PATH}}|$(DEV_BIN_DIR)/thrud-collector|g' \
	    -e 's|{{HOME}}|$(HOME)|g' \
	    dev/$(DEV_PLIST).template > $(DEV_SERVICE_DIR)/$(DEV_PLIST)
	@echo "✅ Service plist installed"

start: install-service
	@echo "🚀 Starting Thrud collector..."
	launchctl load $(DEV_SERVICE_DIR)/$(DEV_PLIST)
	@sleep 1
	@$(MAKE) status

stop:
	@echo "🛑 Stopping Thrud collector..."
	-launchctl unload $(DEV_SERVICE_DIR)/$(DEV_PLIST) 2>/dev/null
	@echo "✅ Collector stopped"

restart: stop start

status:
	@echo "📊 Thrud Status:"
	@echo "==============="
	@echo "Service:"
	@if launchctl list | grep -q thrud; then \
		echo "  ✅ Running"; \
		launchctl list | grep thrud; \
	else \
		echo "  ❌ Not running"; \
	fi
	@echo ""
	@echo "Database:"
	@if [ -f ~/.thrud/thrud.db ]; then \
		echo "  ✅ Database exists: ~/.thrud/thrud.db"; \
		echo "  📏 Size: $$(du -h ~/.thrud/thrud.db | cut -f1)"; \
	else \
		echo "  ❌ No database found"; \
	fi
	@echo ""
	@echo "Recent logs:"
	@if [ -f $(DEV_LOG_DIR)/collector.log ]; then \
		echo "  📝 Last 3 lines:"; \
		tail -3 $(DEV_LOG_DIR)/collector.log | sed 's/^/    /'; \
	else \
		echo "  ❌ No logs found"; \
	fi

logs:
	@echo "📝 Following Thrud logs (Ctrl+C to exit)..."
	@if [ -f $(DEV_LOG_DIR)/collector.log ]; then \
		tail -f $(DEV_LOG_DIR)/collector.log; \
	else \
		echo "❌ No log file found at $(DEV_LOG_DIR)/collector.log"; \
		echo "💡 Try 'make start' first"; \
	fi

clean: stop
	@echo "🧹 Cleaning up development installation..."
	rm -f $(DEV_BIN_DIR)/thrud-*
	rm -f $(DEV_SERVICE_DIR)/$(DEV_PLIST)
	@echo "✅ Cleaned up binaries and service"

uninstall: clean
	@echo "🗑️  Completely removing Thrud..."
	rm -rf ~/.thrud/
	@echo "✅ Database and logs removed"

# Quick development targets
dev-start: build
	@echo "🔧 Starting Thrud in development mode (foreground)..."
	THRUD_DEV_MODE=1 ./target/release/thrud-collector --interval 1.0

dev-fast: build
	@echo "⚡ Starting Thrud with fast collection (100ms)..."
	THRUD_DEV_MODE=1 ./target/release/thrud-collector --interval 0.1