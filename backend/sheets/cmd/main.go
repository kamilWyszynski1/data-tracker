package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"sort"
	"time"

	"github.com/binanceBot/backend/binance"
	"github.com/binanceBot/backend/sheets/api"

	"google.golang.org/api/option"

	"golang.org/x/net/context"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/google"
	"google.golang.org/api/sheets/v4"
)

// Retrieve a token, saves the token, then returns the generated client.
func getClient(config *oauth2.Config) *http.Client {
	// The file token.json stores the user's access and refresh tokens, and is
	// created automatically when the authorization flow completes for the first
	// time.
	tokFile := "token.json"
	tok, err := tokenFromFile(tokFile)
	if err != nil {
		tok = getTokenFromWeb(config)
		saveToken(tokFile, tok)
	}
	return config.Client(context.Background(), tok)
}

// Request a token from the web, then returns the retrieved token.
func getTokenFromWeb(config *oauth2.Config) *oauth2.Token {
	authURL := config.AuthCodeURL("state-token", oauth2.AccessTypeOffline)
	fmt.Printf("Go to the following link in your browser then type the "+
		"authorization code: \n%v\n", authURL)

	var authCode string
	if _, err := fmt.Scan(&authCode); err != nil {
		log.Fatalf("Unable to read authorization code: %v", err)
	}

	tok, err := config.Exchange(context.TODO(), authCode)
	if err != nil {
		log.Fatalf("Unable to retrieve token from web: %v", err)
	}
	return tok
}

// Retrieves a token from a local file.
func tokenFromFile(file string) (*oauth2.Token, error) {
	f, err := os.Open(file)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	tok := &oauth2.Token{}
	err = json.NewDecoder(f).Decode(tok)
	return tok, err
}

// Saves a token to a file path.
func saveToken(path string, token *oauth2.Token) {
	fmt.Printf("Saving credential file to: %s\n", path)
	f, err := os.OpenFile(path, os.O_RDWR|os.O_CREATE|os.O_TRUNC, 0600)
	if err != nil {
		log.Fatalf("Unable to cache oauth token: %v", err)
	}
	defer f.Close()
	json.NewEncoder(f).Encode(token)
}

type pair struct {
	x, y int64
}

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
	client := getClient(config)

	srv, err := sheets.NewService(context.Background(), option.WithHTTPClient(client))
	if err != nil {
		log.Fatalf("Unable to retrieve Sheets client: %v", err)
	}

	// Prints the names and majors of students in a sample spreadsheet:
	// https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit
	spreadsheetId := "1T62QvQrrgNnKNB0JdC24qHm6bYGMgbY4usY8FLFEcec"
	readRange := "A2:A"
	resp, err := srv.Spreadsheets.Values.Get(spreadsheetId, readRange).Do()
	if err != nil {
		log.Fatalf("Unable to retrieve data from sheet: %v", err)
	}

	bCli := binance.NewBinance(http.DefaultClient, "https://api.binance.com", "QbLjiZkYn6mReDrK8wI64uKh2GF42F2ezmigik7prdH212Yi5I5f3wRCTbVWWktm", []byte("5BiEPBLveIXIrNqLX9hqV0QAvYZYNK3TVALk6ZBEdrpBsBGYVl2zeQhDEDZa4jUB"))
	var sells []pair

	if len(resp.Values) == 0 {
		fmt.Println("No data found.")
	} else {
		for i, row := range resp.Values {
			if len(row) == 0 {
				continue
			}
			mtr, err := bCli.MyTrades(binance.MyTradesRequest{
				Symbol:    row[0].(string),
				Timestamp: time.Now().Add(-time.Second),
			})
			if err != nil {
				panic(fmt.Errorf("failed to get MyTrades, %w", err))
			}

			sort.Slice(mtr.Trades, func(i, j int) bool {
				return mtr.Trades[i].Time <= mtr.Trades[j].Time
			})

			prices := make([]interface{}, 0, len(mtr.Trades))
			for j, trade := range mtr.Trades {
				if !trade.Isbuyer {
					sells = append(sells, pair{int64(i), int64(j)})
				}
				prices = append(prices, trade.Price)
			}

			vr := &sheets.ValueRange{}
			vr.Values = append(vr.Values, prices)
			fmt.Println("values: ", prices)
			range_ := fmt.Sprintf("B%d:%s%d", 2+i, string(rune('B'+len(prices)-1)), 2+i)
			fmt.Printf("inserting %s to range: %s\n", row[0], range_)
			_, err = srv.Spreadsheets.Values.Update(spreadsheetId, range_, vr).ValueInputOption("USER_ENTERED").Do()
			if err != nil {
				panic(err)
			}
		}
		reqs := make([]*sheets.Request, 0, len(sells))
		for _, p := range sells {

			reqs = append(reqs, &sheets.Request{
				RepeatCell: &sheets.RepeatCellRequest{
					Cell: &sheets.CellData{
						UserEnteredFormat: &sheets.CellFormat{
							BackgroundColor: api.ColorLightRed2,
						},
					},
					Fields: "userEnteredFormat(backgroundColor)",
					Range: &sheets.GridRange{
						EndColumnIndex:   p.y + 2,
						EndRowIndex:      p.x + 2,
						StartColumnIndex: p.y + 1,
						StartRowIndex:    p.x + 1,
						SheetId:          0,
					},
				},
			})
		}

		batchResp, err := srv.Spreadsheets.BatchUpdate(spreadsheetId, &sheets.BatchUpdateSpreadsheetRequest{
			Requests: reqs,
		}).Do()
		if err != nil {
			panic(err)
		}
		fmt.Println(batchResp)

	}
}
