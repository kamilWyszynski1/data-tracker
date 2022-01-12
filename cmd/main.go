package main

import (
	"context"
	"data-tracker/api"
	"data-tracker/env"
	"data-tracker/tracker"

	"io/ioutil"
	"log"
	"os"
	"time"

	"golang.org/x/oauth2/google"
	"google.golang.org/api/option"
	"google.golang.org/api/sheets/v4"
)

func main() {
	b, err := ioutil.ReadFile(os.Getenv(env.CREDENTIALS_FILE_PATH))
	if err != nil {
		log.Fatalf("Unable to read client secret file: %v", err)
	}

	// If modifying these scopes, delete your previously saved token.json.
	config, err := google.ConfigFromJSON(b, "https://www.googleapis.com/auth/spreadsheets")
	if err != nil {
		log.Fatalf("Unable to parse client secret file to config: %v", err)
	}
	client := api.GetClient(config)

	srv, err := sheets.NewService(context.Background(), option.WithHTTPClient(client))
	if err != nil {
		log.Fatalf("Unable to retrieve Sheets client: %v", err)
	}

	log := log.Default()
	tr := tracker.NewTracker(srv, log)
	tr.AddTrackingFn("A", time.Second*15, func(ctx context.Context) (tracker.TrackedData, error) {
		return []string{"1", "2"}, nil
	}, tracker.WithTimestamp(true))
	tr.Start(context.Background())
	time.Sleep(time.Minute)
}
