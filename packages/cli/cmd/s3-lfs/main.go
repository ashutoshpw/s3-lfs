package main

import (
	"errors"
	"flag"
	"log"
	"os"

	"github.com/ashutoshpw/s3-lfs/packages/cli"
)

func main() {
	if err := cli.Run(os.Args[1:]); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return
		}
		log.Fatal(err)
	}
}
