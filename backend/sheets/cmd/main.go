package main

import (
	"context"
	"io/ioutil"
	"log"
	"os"
	"time"

	"github.com/binanceBot/backend/sheets/api"
	"github.com/binanceBot/backend/sheets/tracker"
	"golang.org/x/oauth2/google"
	"google.golang.org/api/option"
	"google.golang.org/api/sheets/v4"
)

type pair struct {
	x, y int64
}

// spreadsheetID is a spreadsheet ID. This is found in the URL of your sheet.
const spreadsheetID = "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg"

func main() {
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
	// var vr sheets.ValueRange

	// myval := []interface{}{"One", "Two"}
	// vr.Values = append(vr.Values, myval)
	// _, err = srv.Spreadsheets.Values.Update(spreadsheetID, "A1", &vr).ValueInputOption("RAW").Do()
	// if err != nil {
	// 	log.Fatal(err)
	// }

}
