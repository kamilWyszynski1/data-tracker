package common

import (
	"log"
	"os"
)

func EnvOrFatal(key string) string {
	if v := os.Getenv(key); v == "" {
		log.Fatalf("%s is empty", key)
	} else {
		return v
	}
	return ""
}
