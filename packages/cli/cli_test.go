package cli

import (
	"os"
	"testing"

	"github.com/ashutoshpw/s3-lfs/packages/cli/profiles"
)

func TestPrecedenceProfileEnvCLI(t *testing.T) {
	home := t.TempDir()
	oldHome := os.Getenv("HOME")
	if err := os.Setenv("HOME", home); err != nil {
		t.Fatalf("set HOME: %v", err)
	}
	t.Cleanup(func() {
		_ = os.Setenv("HOME", oldHome)
		_ = os.Unsetenv("AWS_ACCESS_KEY_ID")
		_ = os.Unsetenv("AWS_SECRET_ACCESS_KEY")
		_ = os.Unsetenv("S3_BUCKET")
		_ = os.Unsetenv("AWS_REGION")
		_ = os.Unsetenv("AWS_S3_ENDPOINT")
	})

	if err := profiles.Save("dev", &profiles.Profile{
		AccessKeyID:         "profile-ak",
		SecretAccessKey:     "profile-sk",
		Bucket:              "profile-bucket",
		Endpoint:            "https://profile.endpoint",
		Region:              "profile-region",
		RootPath:            "profile-root",
		Compression:         "zstd",
		UsePathStyle:        false,
		DeleteOtherVersions: true,
	}); err != nil {
		t.Fatalf("save profile: %v", err)
	}

	if err := os.Setenv("S3_BUCKET", "env-bucket"); err != nil {
		t.Fatalf("set env: %v", err)
	}
	if err := os.Setenv("AWS_REGION", "env-region"); err != nil {
		t.Fatalf("set env: %v", err)
	}

	resolved, err := runtimeFromProfile("dev")
	if err != nil {
		t.Fatalf("load runtime from profile: %v", err)
	}

	applyEnvOverrides(resolved)
	cli := &runtimeFlags{
		Endpoint:        "https://cli.endpoint",
		RootPath:        "cli-root",
		Compression:     "gzip",
		UsePathStyle:    true,
		AccessKeyID:     "",
		SecretAccessKey: "",
	}
	visited := map[string]bool{
		"endpoint":       true,
		"root_path":      true,
		"compression":    true,
		"use_path_style": true,
	}
	applyCLIOverrides(resolved, cli, visited)

	if resolved.Bucket != "env-bucket" {
		t.Fatalf("expected env bucket override, got %q", resolved.Bucket)
	}
	if resolved.Region != "env-region" {
		t.Fatalf("expected env region override, got %q", resolved.Region)
	}
	if resolved.Endpoint != "https://cli.endpoint" {
		t.Fatalf("expected cli endpoint override, got %q", resolved.Endpoint)
	}
	if resolved.RootPath != "cli-root" {
		t.Fatalf("expected cli root_path override, got %q", resolved.RootPath)
	}
	if resolved.Compression != "gzip" {
		t.Fatalf("expected cli compression override, got %q", resolved.Compression)
	}
	if !resolved.UsePathStyle {
		t.Fatalf("expected cli use_path_style override")
	}
}
