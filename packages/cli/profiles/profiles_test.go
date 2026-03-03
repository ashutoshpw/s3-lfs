package profiles

import (
	"os"
	"path/filepath"
	"testing"
)

func TestValidateSlug(t *testing.T) {
	valid := []string{"dev", "team_1", "prod-01", "A1"}
	for _, slug := range valid {
		if err := ValidateSlug(slug); err != nil {
			t.Fatalf("expected valid slug %q: %v", slug, err)
		}
	}

	invalid := []string{"", "../bad", "bad/path", "bad space", "-bad", "_bad"}
	for _, slug := range invalid {
		if err := ValidateSlug(slug); err == nil {
			t.Fatalf("expected invalid slug %q", slug)
		}
	}
}

func TestSaveLoadListDelete(t *testing.T) {
	oldResolver := resolveUserHome
	t.Cleanup(func() {
		resolveUserHome = oldResolver
	})

	home := t.TempDir()
	resolveUserHome = func() (string, error) {
		return home, nil
	}

	p := &Profile{
		AccessKeyID:         "AKIA...",
		SecretAccessKey:     "secret",
		Bucket:              "my-bucket",
		Endpoint:            "https://s3.example.com",
		Region:              "us-east-1",
		RootPath:            "repo-root",
		Compression:         "zstd",
		UsePathStyle:        true,
		DeleteOtherVersions: true,
	}

	if err := Save("dev", p); err != nil {
		t.Fatalf("save failed: %v", err)
	}

	loaded, err := Load("dev")
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if loaded.Endpoint != p.Endpoint || loaded.Compression != p.Compression || !loaded.UsePathStyle {
		t.Fatalf("unexpected loaded profile: %#v", loaded)
	}

	profiles, err := List()
	if err != nil {
		t.Fatalf("list failed: %v", err)
	}
	if len(profiles) != 1 || profiles[0] != "dev" {
		t.Fatalf("unexpected profile list: %#v", profiles)
	}

	if err := Delete("dev"); err != nil {
		t.Fatalf("delete failed: %v", err)
	}
	profiles, err = List()
	if err != nil {
		t.Fatalf("list after delete failed: %v", err)
	}
	if len(profiles) != 0 {
		t.Fatalf("expected empty profile list, got %#v", profiles)
	}

	path := filepath.Join(home, ".config", "s3-lfs", "profiles", "dev")
	if _, err := os.Stat(path); !os.IsNotExist(err) {
		t.Fatalf("expected profile dir removed, stat err: %v", err)
	}
}

func TestLoadDefaultsDeleteOtherVersionsWhenMissing(t *testing.T) {
	oldResolver := resolveUserHome
	t.Cleanup(func() {
		resolveUserHome = oldResolver
	})

	home := t.TempDir()
	resolveUserHome = func() (string, error) {
		return home, nil
	}

	dir := filepath.Join(home, ".config", "s3-lfs", "profiles", "legacy")
	if err := os.MkdirAll(dir, 0o700); err != nil {
		t.Fatalf("mkdir failed: %v", err)
	}
	content := `{
  "access_key_id": "AKIA...",
  "secret_access_key": "secret",
  "bucket": "b",
  "endpoint": "https://s3.example.com",
  "region": "us-east-1",
  "root_path": "",
  "compression": "zstd",
  "use_path_style": false
}
`
	if err := os.WriteFile(filepath.Join(dir, "credentials.json"), []byte(content), 0o600); err != nil {
		t.Fatalf("write failed: %v", err)
	}

	loaded, err := Load("legacy")
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if !loaded.DeleteOtherVersions {
		t.Fatalf("expected delete_other_versions default true")
	}
}

func TestLoadDefaultsCompressionWhenMissing(t *testing.T) {
	oldResolver := resolveUserHome
	t.Cleanup(func() {
		resolveUserHome = oldResolver
	})

	home := t.TempDir()
	resolveUserHome = func() (string, error) {
		return home, nil
	}

	dir := filepath.Join(home, ".config", "s3-lfs", "profiles", "legacy-compression")
	if err := os.MkdirAll(dir, 0o700); err != nil {
		t.Fatalf("mkdir failed: %v", err)
	}
	content := `{
  "access_key_id": "AKIA...",
  "secret_access_key": "secret",
  "bucket": "b",
  "endpoint": "https://s3.example.com",
  "region": "us-east-1",
  "root_path": "",
  "use_path_style": false,
  "delete_other_versions": true
}
`
	if err := os.WriteFile(filepath.Join(dir, "credentials.json"), []byte(content), 0o600); err != nil {
		t.Fatalf("write failed: %v", err)
	}

	loaded, err := Load("legacy-compression")
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if loaded.Compression != DefaultCompression {
		t.Fatalf("expected default compression %q, got %q", DefaultCompression, loaded.Compression)
	}
}

func TestValidateProfileRequiresRegion(t *testing.T) {
	p := &Profile{
		Bucket:      "b",
		Endpoint:    "https://s3.example.com",
		Compression: "none",
	}
	if err := ValidateProfile(p); err == nil {
		t.Fatalf("expected region validation error")
	}
}
