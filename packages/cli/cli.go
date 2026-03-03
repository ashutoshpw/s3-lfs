package cli

import (
	"bufio"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/ashutoshpw/s3-lfs/packages/cli/compression"
	"github.com/ashutoshpw/s3-lfs/packages/cli/profiles"
	"github.com/ashutoshpw/s3-lfs/packages/cli/s3adapter"
	"github.com/ashutoshpw/s3-lfs/packages/cli/service"
)

var defaultCompression = profiles.DefaultCompression

func compressionByName(name string) (compression.Compression, bool) {
	for _, c := range compression.Compressions {
		if c.Name() == name {
			return c, true
		}
	}
	return nil, false
}

type runtimeFlags struct {
	Profile             string
	AccessKeyID         string
	SecretAccessKey     string
	Bucket              string
	Endpoint            string
	Region              string
	RootPath            string
	UsePathStyle        bool
	DeleteOtherVersions bool
	Compression         string
}

func newRuntimeFlagSet(name string, target *runtimeFlags) *flag.FlagSet {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.StringVar(&target.Profile, "profile", "", "Named S3 profile slug from ~/.config/s3-lfs/profiles/<slug>/credentials.json")
	fs.StringVar(&target.AccessKeyID, "access_key_id", "", "S3 Access Key ID")
	fs.StringVar(&target.SecretAccessKey, "secret_access_key", "", "S3 Secret Access Key")
	fs.StringVar(&target.Bucket, "bucket", "", "S3 Bucket")
	fs.StringVar(&target.Endpoint, "endpoint", "", "S3 Endpoint")
	fs.StringVar(&target.Region, "region", "", "S3 Region")
	fs.StringVar(&target.RootPath, "root_path", "", "Path within the bucket under which LFS files are uploaded. Can be empty.")
	fs.BoolVar(&target.UsePathStyle, "use_path_style", false, "Whether to use path-style URLs for S3.")
	fs.BoolVar(&target.DeleteOtherVersions, "delete_other_versions", true, "Whether to delete other (e.g. uploaded using different compression methods) versions of the stored file after upload.")

	var compressions []string
	for _, c := range compression.Compressions {
		compressions = append(compressions, c.Name())
	}
	fs.StringVar(&target.Compression, "compression", defaultCompression, "Compression to use for storing files. Possible values: "+strings.Join(compressions, ", "))
	return fs
}

func visitedFlagNames(fs *flag.FlagSet) map[string]bool {
	set := map[string]bool{}
	fs.Visit(func(f *flag.Flag) {
		set[f.Name] = true
	})
	return set
}

func applyEnvOverrides(f *runtimeFlags) {
	if value := os.Getenv("AWS_ACCESS_KEY_ID"); value != "" {
		f.AccessKeyID = value
	}
	if value := os.Getenv("AWS_SECRET_ACCESS_KEY"); value != "" {
		f.SecretAccessKey = value
	}
	if value := os.Getenv("S3_BUCKET"); value != "" {
		f.Bucket = value
	}
	if value := os.Getenv("AWS_REGION"); value != "" {
		f.Region = value
	}
	if value := os.Getenv("AWS_S3_ENDPOINT"); value != "" {
		f.Endpoint = value
	}
}

func applyCLIOverrides(dst *runtimeFlags, src *runtimeFlags, visited map[string]bool) {
	if visited["access_key_id"] {
		dst.AccessKeyID = src.AccessKeyID
	}
	if visited["secret_access_key"] {
		dst.SecretAccessKey = src.SecretAccessKey
	}
	if visited["bucket"] {
		dst.Bucket = src.Bucket
	}
	if visited["endpoint"] {
		dst.Endpoint = src.Endpoint
	}
	if visited["region"] {
		dst.Region = src.Region
	}
	if visited["root_path"] {
		dst.RootPath = src.RootPath
	}
	if visited["use_path_style"] {
		dst.UsePathStyle = src.UsePathStyle
	}
	if visited["delete_other_versions"] {
		dst.DeleteOtherVersions = src.DeleteOtherVersions
	}
	if visited["compression"] {
		dst.Compression = src.Compression
	}
}

func runtimeFromProfile(slug string) (*runtimeFlags, error) {
	p, err := profiles.Load(slug)
	if err != nil {
		return nil, err
	}
	return &runtimeFlags{
		Profile:             slug,
		AccessKeyID:         p.AccessKeyID,
		SecretAccessKey:     p.SecretAccessKey,
		Bucket:              p.Bucket,
		Endpoint:            p.Endpoint,
		Region:              p.Region,
		RootPath:            p.RootPath,
		UsePathStyle:        p.UsePathStyle,
		DeleteOtherVersions: p.DeleteOtherVersions,
		Compression:         p.Compression,
	}, nil
}

func toS3Config(f *runtimeFlags) (*s3adapter.Config, error) {
	comp, ok := compressionByName(f.Compression)
	if !ok {
		return nil, fmt.Errorf("invalid compression set: %s", f.Compression)
	}
	return &s3adapter.Config{
		AccessKeyId:         f.AccessKeyID,
		SecretAccessKey:     f.SecretAccessKey,
		Bucket:              f.Bucket,
		Endpoint:            f.Endpoint,
		Region:              f.Region,
		RootPath:            f.RootPath,
		UsePathStyle:        f.UsePathStyle,
		DeleteOtherVersions: f.DeleteOtherVersions,
		Compression:         comp,
	}, nil
}

func runTransferAgent(args []string) error {
	parsed := runtimeFlags{Compression: defaultCompression, DeleteOtherVersions: true}
	fs := newRuntimeFlagSet("s3-lfs", &parsed)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(fs.Args()) > 0 {
		return fmt.Errorf("unexpected positional arguments: %s", strings.Join(fs.Args(), " "))
	}

	resolved := runtimeFlags{Compression: defaultCompression, DeleteOtherVersions: true}
	if parsed.Profile != "" {
		fromProfile, err := runtimeFromProfile(parsed.Profile)
		if err != nil {
			return fmt.Errorf("load profile %q: %w", parsed.Profile, err)
		}
		resolved = *fromProfile
	}

	visited := visitedFlagNames(fs)
	cwd, err := os.Getwd()
	if err != nil {
		return err
	}
	repoCfg, err := resolveRepoConfig(cwd)
	if err != nil && !errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("load repo .lfsconfig: %w", err)
	}
	if err == nil {
		applyRepoOverrides(&resolved, repoCfg, visited)
	}

	// Precedence:
	// - root_path/compression: profile < repo .lfsconfig < explicit CLI flags
	// - all other settings: profile < environment < explicit CLI flags
	applyEnvOverrides(&resolved)
	applyCLIOverrides(&resolved, &parsed, visited)
	if resolved.Compression == "" {
		resolved.Compression = defaultCompression
	}
	if resolved.Profile == "" {
		resolved.Profile = parsed.Profile
	}

	cfg, err := toS3Config(&resolved)
	if err != nil {
		return err
	}

	return service.Serve(os.Stdin, os.Stdout, os.Stderr, cfg)
}

func prompt(reader *bufio.Reader, writer io.Writer, label, current string, required bool) (string, error) {
	for {
		if current == "" {
			fmt.Fprintf(writer, "%s: ", label)
		} else {
			fmt.Fprintf(writer, "%s [%s]: ", label, current)
		}
		line, err := reader.ReadString('\n')
		if err != nil {
			if errors.Is(err, io.EOF) {
				line = strings.TrimSpace(line)
				if line == "" {
					if current != "" {
						return current, nil
					}
					if required {
						return "", fmt.Errorf("%s is required", label)
					}
					return "", nil
				}
			} else {
				return "", err
			}
		} else {
			line = strings.TrimSpace(line)
		}
		if line == "" {
			if current != "" {
				return current, nil
			}
			if required {
				fmt.Fprintln(writer, "Value is required.")
				continue
			}
		}
		if line == "" {
			return "", nil
		}
		return line, nil
	}
}

func promptBool(reader *bufio.Reader, writer io.Writer, label string, current bool) (bool, error) {
	for {
		defaultValue := "n"
		if current {
			defaultValue = "y"
		}
		fmt.Fprintf(writer, "%s [y/n, default=%s]: ", label, defaultValue)
		line, err := reader.ReadString('\n')
		if err != nil {
			if errors.Is(err, io.EOF) {
				line = strings.TrimSpace(line)
			} else {
				return false, err
			}
		} else {
			line = strings.TrimSpace(line)
		}
		if line == "" {
			return current, nil
		}
		switch strings.ToLower(line) {
		case "y", "yes", "true", "1":
			return true, nil
		case "n", "no", "false", "0":
			return false, nil
		default:
			fmt.Fprintln(writer, "Please enter y or n.")
		}
	}
}

func runSetup(args []string) error {
	parsed := runtimeFlags{Compression: defaultCompression, DeleteOtherVersions: true}
	fs := newRuntimeFlagSet("setup", &parsed)
	if err := fs.Parse(args); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			out := fs.Output()
			fmt.Fprintln(out)
			fmt.Fprintln(out, "Setup is interactive. If --profile is omitted, you can select an existing profile to edit or add a new profile.")
		}
		return err
	}
	if len(fs.Args()) > 0 {
		return fmt.Errorf("unexpected positional arguments: %s", strings.Join(fs.Args(), " "))
	}
	reader := bufio.NewReader(os.Stdin)
	stdinInfo, err := os.Stdin.Stat()
	if err != nil {
		return err
	}
	isTTY := stdinInfo.Mode()&os.ModeCharDevice != 0

	selectedProfile := parsed.Profile
	if selectedProfile == "" {
		if !isTTY {
			return errors.New("--profile is required when stdin is not a terminal")
		}
		selectedProfile, err = selectProfileForSetup(reader, os.Stdout)
		if err != nil {
			return err
		}
	}
	if err := profiles.ValidateSlug(selectedProfile); err != nil {
		return err
	}

	visited := visitedFlagNames(fs)

	current := runtimeFlags{Profile: selectedProfile, Compression: defaultCompression, DeleteOtherVersions: true}
	if existing, err := runtimeFromProfile(selectedProfile); err == nil {
		current = *existing
	}

	applyCLIOverrides(&current, &parsed, visited)

	hasConfigFlag := false
	for _, flagName := range []string{"access_key_id", "secret_access_key", "bucket", "endpoint", "region", "root_path", "use_path_style", "delete_other_versions", "compression"} {
		if visited[flagName] {
			hasConfigFlag = true
			break
		}
	}

	if !hasConfigFlag {
		if !isTTY {
			return errors.New("interactive setup requires a terminal; pass flags for non-interactive setup")
		}
		current.AccessKeyID, err = prompt(reader, os.Stdout, "S3 access key ID (optional)", current.AccessKeyID, false)
		if err != nil {
			return err
		}
		current.SecretAccessKey, err = prompt(reader, os.Stdout, "S3 secret access key (optional)", current.SecretAccessKey, false)
		if err != nil {
			return err
		}
		current.Bucket, err = prompt(reader, os.Stdout, "S3 bucket", current.Bucket, true)
		if err != nil {
			return err
		}
		current.Endpoint, err = prompt(reader, os.Stdout, "S3 endpoint", current.Endpoint, true)
		if err != nil {
			return err
		}
		current.Region, err = prompt(reader, os.Stdout, "S3 region", current.Region, true)
		if err != nil {
			return err
		}
		current.RootPath, err = prompt(reader, os.Stdout, "Root path in bucket (optional)", current.RootPath, false)
		if err != nil {
			return err
		}
		current.Compression, err = prompt(reader, os.Stdout, "Compression (zstd|gzip|none)", current.Compression, true)
		if err != nil {
			return err
		}
		current.UsePathStyle, err = promptBool(reader, os.Stdout, "Use path-style URLs", current.UsePathStyle)
		if err != nil {
			return err
		}
		current.DeleteOtherVersions, err = promptBool(reader, os.Stdout, "Delete files uploaded with other compression", current.DeleteOtherVersions)
		if err != nil {
			return err
		}
	} else {
		// Hybrid mode: if required fields are still missing and stdin is interactive, ask only for missing required values.
		if !isTTY && (current.Bucket == "" || current.Endpoint == "" || current.Region == "") {
			return errors.New("bucket, endpoint, and region are required; pass them as flags or run interactive setup in a terminal")
		}
		if isTTY && current.Bucket == "" {
			current.Bucket, err = prompt(reader, os.Stdout, "S3 bucket", current.Bucket, true)
			if err != nil {
				return err
			}
		}
		if isTTY && current.Endpoint == "" {
			current.Endpoint, err = prompt(reader, os.Stdout, "S3 endpoint", current.Endpoint, true)
			if err != nil {
				return err
			}
		}
		if isTTY && current.Region == "" {
			current.Region, err = prompt(reader, os.Stdout, "S3 region", current.Region, true)
			if err != nil {
				return err
			}
		}
	}

	p := &profiles.Profile{
		AccessKeyID:         current.AccessKeyID,
		SecretAccessKey:     current.SecretAccessKey,
		Bucket:              current.Bucket,
		Endpoint:            current.Endpoint,
		Region:              current.Region,
		RootPath:            current.RootPath,
		Compression:         current.Compression,
		UsePathStyle:        current.UsePathStyle,
		DeleteOtherVersions: current.DeleteOtherVersions,
	}
	if err := profiles.Save(selectedProfile, p); err != nil {
		return err
	}

	fmt.Printf("Saved profile %q\n", selectedProfile)
	return nil
}

func selectProfileForSetup(reader *bufio.Reader, writer io.Writer) (string, error) {
	items, err := profiles.List()
	if err != nil {
		return "", err
	}
	if len(items) == 0 {
		fmt.Fprintln(writer, "No existing profiles found. Creating a new profile.")
		return promptProfileSlug(reader, writer)
	}

	for {
		fmt.Fprintln(writer, "Select a profile for setup:")
		fmt.Fprintln(writer, "  1) Add new profile")
		for i, slug := range items {
			fmt.Fprintf(writer, "  %d) Edit %s\n", i+2, slug)
		}
		choice, err := prompt(reader, writer, "Choice", "1", true)
		if err != nil {
			return "", err
		}
		n, err := strconv.Atoi(choice)
		if err != nil {
			fmt.Fprintf(writer, "Invalid choice %q. Enter a number.\n", choice)
			continue
		}
		if n == 1 {
			return promptProfileSlug(reader, writer)
		}
		index := n - 2
		if index >= 0 && index < len(items) {
			return items[index], nil
		}
		fmt.Fprintf(writer, "Invalid choice %d. Enter a number between 1 and %d.\n", n, len(items)+1)
	}
}

func promptProfileSlug(reader *bufio.Reader, writer io.Writer) (string, error) {
	for {
		slug, err := prompt(reader, writer, "Profile slug", "", true)
		if err != nil {
			return "", err
		}
		if err := profiles.ValidateSlug(slug); err != nil {
			fmt.Fprintf(writer, "%v\n", err)
			continue
		}
		return slug, nil
	}
}

func runProfile(args []string) error {
	if len(args) == 0 {
		return errors.New("expected one of: list, show, delete")
	}
	switch args[0] {
	case "list":
		return runProfileList(args[1:])
	case "show":
		return runProfileShow(args[1:])
	case "delete":
		return runProfileDelete(args[1:])
	default:
		return fmt.Errorf("unknown profile command %q", args[0])
	}
}

func runProfileList(args []string) error {
	fs := flag.NewFlagSet("profile list", flag.ContinueOnError)
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(fs.Args()) > 0 {
		return fmt.Errorf("unexpected positional arguments: %s", strings.Join(fs.Args(), " "))
	}
	items, err := profiles.List()
	if err != nil {
		return err
	}
	for _, item := range items {
		fmt.Println(item)
	}
	return nil
}

func runProfileShow(args []string) error {
	fs := flag.NewFlagSet("profile show", flag.ContinueOnError)
	var slug string
	fs.StringVar(&slug, "profile", "", "Profile slug")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(fs.Args()) > 0 {
		return fmt.Errorf("unexpected positional arguments: %s", strings.Join(fs.Args(), " "))
	}
	if slug == "" {
		return errors.New("--profile is required")
	}
	p, err := profiles.Load(slug)
	if err != nil {
		return err
	}
	payload, err := json.MarshalIndent(p, "", "  ")
	if err != nil {
		return err
	}
	fmt.Printf("%s\n", payload)
	return nil
}

func runProfileDelete(args []string) error {
	fs := flag.NewFlagSet("profile delete", flag.ContinueOnError)
	var slug string
	fs.StringVar(&slug, "profile", "", "Profile slug")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if len(fs.Args()) > 0 {
		return fmt.Errorf("unexpected positional arguments: %s", strings.Join(fs.Args(), " "))
	}
	if slug == "" {
		return errors.New("--profile is required")
	}
	if err := profiles.Delete(slug); err != nil {
		return err
	}
	fmt.Printf("Deleted profile %q\n", slug)
	return nil
}

func printUsage(w io.Writer) {
	fmt.Fprintln(w, "Usage:")
	fmt.Fprintln(w, "  s3-lfs [flags]")
	fmt.Fprintln(w, "  s3-lfs setup [--profile <slug>] [flags]")
	fmt.Fprintln(w, "  s3-lfs profile list")
	fmt.Fprintln(w, "  s3-lfs profile show --profile <slug>")
	fmt.Fprintln(w, "  s3-lfs profile delete --profile <slug>")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "Without a subcommand, s3-lfs runs as a Git LFS custom transfer agent.")
	fmt.Fprintln(w, "Setup is interactive:")
	fmt.Fprintln(w, "  - s3-lfs setup: choose existing profile to edit or add a new profile")
	fmt.Fprintln(w, "  - s3-lfs setup --profile <slug>: edit/create a specific profile")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "Transfer-agent/setup flags:")
	flags := runtimeFlags{Compression: defaultCompression, DeleteOtherVersions: true}
	fs := newRuntimeFlagSet("s3-lfs", &flags)
	fs.SetOutput(w)
	fs.PrintDefaults()
	fmt.Fprintln(w)
	fmt.Fprintln(w, "Profile commands:")
	fmt.Fprintln(w, "  list:   list configured profiles")
	fmt.Fprintln(w, "  show:   print profile JSON")
	fmt.Fprintln(w, "  delete: delete a profile directory")
}

func Run(args []string) error {
	if len(args) > 0 {
		switch args[0] {
		case "-h", "--help", "help":
			printUsage(os.Stdout)
			return nil
		}
		switch args[0] {
		case "setup":
			return runSetup(args[1:])
		case "profile":
			return runProfile(args[1:])
		}
	}
	return runTransferAgent(args)
}
