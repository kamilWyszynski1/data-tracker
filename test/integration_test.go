package integration_test

import (
	"context"
	"data-tracker/api"
	"data-tracker/tracker"
	"io/ioutil"
	"log"
	"os"
	"testing"
	"time"

	"golang.org/x/oauth2/google"
	"google.golang.org/api/option"
	"google.golang.org/api/sheets/v4"
)

// TestTracking tests if tracking works.
func TestTracking(t *testing.T) {
	b, err := ioutil.ReadFile(os.Getenv("CREDENTIALS_FILE"))
	if err != nil {
		log.Fatalf("Unable to read client secret file: %v", err)
	}

	// If modifying these scopes, delete your previously saved token.json.
	config, err := google.ConfigFromJSON(b, "https://www.googleapis.com/auth/spreadsheets")
	if err != nil {
		log.Fatalf("Unable to parse client secret file to config: %v", err)
	}
	client := api.GetClient(config)

	ctx := context.Background()

	srv, err := sheets.NewService(ctx, option.WithHTTPClient(client))
	if err != nil {
		log.Fatalf("Unable to retrieve Sheets client: %v", err)
	}

	ctx, cancel := context.WithCancel(ctx)

	tr := tracker.NewTracker(srv, log.Default())
	tr.AddTrackingFn(
		tracker.Direction("A"),
		time.Second,
		func(ctx context.Context) (tracker.TrackedData, error) {
			cancel()
			return tracker.TrackedData{"elo"}, nil
		},
	)
	tr.Start(ctx)

}
