package cli

import (
	"bufio"
	"errors"
	"os"
	"path/filepath"
	"strings"
)

type repoConfig struct {
	HasRootPath    bool
	RootPath       string
	HasCompression bool
	Compression    string
}

func resolveRepoConfig(startDir string) (*repoConfig, error) {
	repoRoot, err := findRepoRoot(startDir)
	if err != nil {
		return nil, err
	}
	return parseLFSConfig(filepath.Join(repoRoot, ".lfsconfig"))
}

func findRepoRoot(startDir string) (string, error) {
	dir := startDir
	for {
		marker := filepath.Join(dir, ".git")
		if _, err := os.Stat(marker); err == nil {
			return dir, nil
		} else if !errors.Is(err, os.ErrNotExist) {
			return "", err
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			return "", os.ErrNotExist
		}
		dir = parent
	}
}

func parseLFSConfig(path string) (*repoConfig, error) {
	f, err := os.Open(path)
	if errors.Is(err, os.ErrNotExist) {
		return &repoConfig{}, nil
	}
	if err != nil {
		return nil, err
	}
	defer f.Close()

	cfg := &repoConfig{}
	scanner := bufio.NewScanner(f)
	section := ""
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") || strings.HasPrefix(line, ";") {
			continue
		}

		if strings.HasPrefix(line, "[") && strings.HasSuffix(line, "]") {
			section = strings.ToLower(strings.TrimSpace(line[1 : len(line)-1]))
			continue
		}
		if section != "s3-lfs" {
			continue
		}

		key, value, ok := parseConfigKV(line)
		if !ok {
			continue
		}
		key = strings.ToLower(strings.ReplaceAll(strings.TrimSpace(key), "-", "_"))
		value = trimConfigValue(value)

		switch key {
		case "root_path":
			cfg.RootPath = value
			cfg.HasRootPath = true
		case "compression":
			cfg.Compression = strings.ToLower(value)
			cfg.HasCompression = true
		}
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return cfg, nil
}

func parseConfigKV(line string) (string, string, bool) {
	if idx := strings.Index(line, "="); idx >= 0 {
		key := strings.TrimSpace(line[:idx])
		if key == "" {
			return "", "", false
		}
		return key, strings.TrimSpace(line[idx+1:]), true
	}

	fields := strings.Fields(line)
	if len(fields) < 2 {
		return "", "", false
	}
	return fields[0], strings.Join(fields[1:], " "), true
}

func trimConfigValue(value string) string {
	if len(value) >= 2 {
		if (value[0] == '\'' && value[len(value)-1] == '\'') || (value[0] == '"' && value[len(value)-1] == '"') {
			return value[1 : len(value)-1]
		}
	}
	return value
}

func applyRepoOverrides(dst *runtimeFlags, repo *repoConfig, visited map[string]bool) {
	if repo == nil {
		return
	}
	if !visited["root_path"] && repo.HasRootPath {
		dst.RootPath = repo.RootPath
	}
	if !visited["compression"] && repo.HasCompression {
		dst.Compression = repo.Compression
	}
}
