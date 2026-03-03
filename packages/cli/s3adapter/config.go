package s3adapter

import (
	"context"

	"github.com/ashutoshpw/s3-lfs/packages/cli/compression"
	"github.com/aws/aws-sdk-go-v2/aws"
)

type Config struct {
	AccessKeyId         string
	SecretAccessKey     string
	Bucket              string
	Endpoint            string
	Region              string
	RootPath            string
	UsePathStyle        bool
	Compression         compression.Compression
	DeleteOtherVersions bool
}

func (config *Config) Retrieve(context.Context) (aws.Credentials, error) {
	return aws.Credentials{Source: "s3-lfs",
		AccessKeyID:     config.AccessKeyId,
		SecretAccessKey: config.SecretAccessKey,
	}, nil
}
