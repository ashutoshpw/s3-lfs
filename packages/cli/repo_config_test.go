package cli

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
)

func TestFindRepoRootWalksUp(t *testing.T) {
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, ".git"), 0o755); err != nil {
		t.Fatalf("mkdir .git: %v", err)
	}
	nested := filepath.Join(root, "a", "b", "c")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("mkdir nested: %v", err)
	}

	got, err := findRepoRoot(nested)
	if err != nil {
		t.Fatalf("findRepoRoot: %v", err)
	}
	if got != root {
		t.Fatalf("expected %q, got %q", root, got)
	}
}

func TestFindRepoRootNotFound(t *testing.T) {
	_, err := findRepoRoot(t.TempDir())
	if !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("expected os.ErrNotExist, got %v", err)
	}
}

func TestParseLFSConfigReadsRootPath(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, ".lfsconfig")
	content := `
[lfs]
url = https://example.com/lfs

[s3-lfs]
root_path = team/repo-a
compression = gzip
`
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write .lfsconfig: %v", err)
	}

	cfg, err := parseLFSConfig(path)
	if err != nil {
		t.Fatalf("parseLFSConfig: %v", err)
	}
	if !cfg.HasRootPath {
		t.Fatalf("expected HasRootPath=true")
	}
	if cfg.RootPath != "team/repo-a" {
		t.Fatalf("expected root_path team/repo-a, got %q", cfg.RootPath)
	}
	if !cfg.HasCompression || cfg.Compression != "gzip" {
		t.Fatalf("expected compression gzip, got %#v", cfg)
	}
}

func TestParseLFSConfigMissingFile(t *testing.T) {
	cfg, err := parseLFSConfig(filepath.Join(t.TempDir(), ".lfsconfig"))
	if err != nil {
		t.Fatalf("parseLFSConfig missing file: %v", err)
	}
	if cfg.HasRootPath {
		t.Fatalf("expected HasRootPath=false")
	}
}

func TestResolveRepoConfigFromNestedDir(t *testing.T) {
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, ".git"), 0o755); err != nil {
		t.Fatalf("mkdir .git: %v", err)
	}
	if err := os.WriteFile(filepath.Join(root, ".lfsconfig"), []byte("[s3-lfs]\nroot_path = demo/root\ncompression = zstd\n"), 0o644); err != nil {
		t.Fatalf("write .lfsconfig: %v", err)
	}
	nested := filepath.Join(root, "x", "y")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("mkdir nested: %v", err)
	}

	cfg, err := resolveRepoConfig(nested)
	if err != nil {
		t.Fatalf("resolveRepoConfig: %v", err)
	}
	if !cfg.HasRootPath || cfg.RootPath != "demo/root" || !cfg.HasCompression || cfg.Compression != "zstd" {
		t.Fatalf("unexpected cfg: %#v", cfg)
	}
}

func TestApplyRepoOverridesForRootPath(t *testing.T) {
	runtime := &runtimeFlags{RootPath: "from-profile"}
	repo := &repoConfig{HasRootPath: true, RootPath: "from-lfsconfig"}

	applyRepoOverrides(runtime, repo, map[string]bool{})
	if runtime.RootPath != "from-lfsconfig" {
		t.Fatalf("expected repo override, got %q", runtime.RootPath)
	}

	runtime.RootPath = "from-profile"
	applyRepoOverrides(runtime, repo, map[string]bool{"root_path": true})
	if runtime.RootPath != "from-profile" {
		t.Fatalf("expected CLI root_path to prevent repo override, got %q", runtime.RootPath)
	}
}

func TestApplyRepoOverridesAllowsEmptyRootPath(t *testing.T) {
	runtime := &runtimeFlags{RootPath: "from-profile"}
	repo := &repoConfig{HasRootPath: true, RootPath: ""}

	applyRepoOverrides(runtime, repo, map[string]bool{})
	if runtime.RootPath != "" {
		t.Fatalf("expected empty root_path override, got %q", runtime.RootPath)
	}
}

func TestApplyRepoOverridesForCompression(t *testing.T) {
	runtime := &runtimeFlags{Compression: "none"}
	repo := &repoConfig{HasCompression: true, Compression: "gzip"}

	applyRepoOverrides(runtime, repo, map[string]bool{})
	if runtime.Compression != "gzip" {
		t.Fatalf("expected repo compression override, got %q", runtime.Compression)
	}

	runtime.Compression = "none"
	applyRepoOverrides(runtime, repo, map[string]bool{"compression": true})
	if runtime.Compression != "none" {
		t.Fatalf("expected CLI compression to prevent repo override, got %q", runtime.Compression)
	}
}
