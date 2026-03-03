BIN_DIR ?= bin
GO_CMD_PATH ?= ./packages/cli/cmd/s3-lfs
RUST_MANIFEST ?= ./packages/cli-rs/Cargo.toml
RUST_RELEASE_BIN ?= ./packages/cli-rs/target/release/s3-lfs

GO_BIN ?= $(BIN_DIR)/s3-lfs-go
RUST_BIN ?= $(BIN_DIR)/s3-lfs

.PHONY: all build build-go build-rust clean size

all: build

build: build-go build-rust

$(BIN_DIR):
	@mkdir -p $(BIN_DIR)

build-go: | $(BIN_DIR)
	@go build -o $(GO_BIN) $(GO_CMD_PATH)
	@echo "Built $(GO_BIN)"

build-rust: | $(BIN_DIR)
	@cargo build --release --manifest-path $(RUST_MANIFEST)
	@cp $(RUST_RELEASE_BIN) $(RUST_BIN)
	@chmod +x $(RUST_BIN)
	@echo "Built $(RUST_BIN)"

size: build
	@echo "Binary sizes:"
	@ls -lh $(GO_BIN) $(RUST_BIN)

clean:
	@rm -rf $(BIN_DIR)
	@echo "Removed $(BIN_DIR)/"
