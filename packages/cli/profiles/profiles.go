package profiles

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"

	"github.com/ashutoshpw/s3-lfs/packages/cli/compression"
)

const (
	configDir = ".config/s3-lfs/profiles"
	fileName  = "credentials.json"
	// DefaultCompression is used when compression isn't explicitly set.
	DefaultCompression = "none"
)

var (
	slugRegex       = regexp.MustCompile(`^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$`)
	resolveUserHome = os.UserHomeDir
)

type Profile struct {
	AccessKeyID         string `json:"access_key_id"`
	SecretAccessKey     string `json:"secret_access_key"`
	Bucket              string `json:"bucket"`
	Endpoint            string `json:"endpoint"`
	Region              string `json:"region"`
	RootPath            string `json:"root_path"`
	Compression         string `json:"compression"`
	UsePathStyle        bool   `json:"use_path_style"`
	DeleteOtherVersions bool   `json:"delete_other_versions"`
}

func ValidateSlug(slug string) error {
	if !slugRegex.MatchString(slug) {
		return fmt.Errorf("invalid profile slug %q", slug)
	}
	return nil
}

func ValidateProfile(p *Profile) error {
	if p == nil {
		return errors.New("profile is nil")
	}
	if p.Bucket == "" {
		return errors.New("bucket is required")
	}
	if p.Endpoint == "" {
		return errors.New("endpoint is required")
	}
	if p.Region == "" {
		return errors.New("region is required")
	}
	if (p.AccessKeyID == "") != (p.SecretAccessKey == "") {
		return errors.New("access key and secret key should either both be set or both be empty")
	}
	if p.Compression == "" {
		return errors.New("compression is required")
	}
	for _, c := range compression.Compressions {
		if c.Name() == p.Compression {
			return nil
		}
	}
	return fmt.Errorf("invalid compression %q", p.Compression)
}

func baseDir() (string, error) {
	home, err := resolveUserHome()
	if err != nil {
		return "", err
	}
	return filepath.Join(home, configDir), nil
}

func profileDir(slug string) (string, error) {
	if err := ValidateSlug(slug); err != nil {
		return "", err
	}
	base, err := baseDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(base, slug), nil
}

func profilePath(slug string) (string, error) {
	dir, err := profileDir(slug)
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, fileName), nil
}

func Save(slug string, p *Profile) error {
	if err := ValidateSlug(slug); err != nil {
		return err
	}
	if err := ValidateProfile(p); err != nil {
		return err
	}
	path, err := profilePath(slug)
	if err != nil {
		return err
	}
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return err
	}

	payload, err := json.MarshalIndent(p, "", "  ")
	if err != nil {
		return err
	}
	payload = append(payload, '\n')

	tmpFile, err := os.CreateTemp(dir, ".credentials-*.json")
	if err != nil {
		return err
	}
	tmpPath := tmpFile.Name()
	cleanup := func() {
		tmpFile.Close()
		_ = os.Remove(tmpPath)
	}

	if _, err := tmpFile.Write(payload); err != nil {
		cleanup()
		return err
	}
	if err := tmpFile.Chmod(0o600); err != nil {
		cleanup()
		return err
	}
	if err := tmpFile.Close(); err != nil {
		cleanup()
		return err
	}
	if err := os.Rename(tmpPath, path); err != nil {
		_ = os.Remove(tmpPath)
		return err
	}
	return nil
}

func Load(slug string) (*Profile, error) {
	path, err := profilePath(slug)
	if err != nil {
		return nil, err
	}
	content, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var p Profile
	if err := json.Unmarshal(content, &p); err != nil {
		return nil, err
	}
	if p.DeleteOtherVersions == false {
		// Keep default behavior for older files missing this field.
		if !jsonContainsField(content, "delete_other_versions") {
			p.DeleteOtherVersions = true
		}
	}
	if p.Compression == "" && !jsonContainsField(content, "compression") {
		p.Compression = DefaultCompression
	}
	if err := ValidateProfile(&p); err != nil {
		return nil, err
	}
	return &p, nil
}

func jsonContainsField(content []byte, field string) bool {
	var m map[string]json.RawMessage
	if err := json.Unmarshal(content, &m); err != nil {
		return false
	}
	_, ok := m[field]
	return ok
}

func List() ([]string, error) {
	base, err := baseDir()
	if err != nil {
		return nil, err
	}
	entries, err := os.ReadDir(base)
	if errors.Is(err, os.ErrNotExist) {
		return []string{}, nil
	}
	if err != nil {
		return nil, err
	}

	var ret []string
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		slug := entry.Name()
		if ValidateSlug(slug) == nil {
			ret = append(ret, slug)
		}
	}
	sort.Strings(ret)
	return ret, nil
}

func Delete(slug string) error {
	dir, err := profileDir(slug)
	if err != nil {
		return err
	}
	if err := os.RemoveAll(dir); err != nil {
		return err
	}
	return nil
}
