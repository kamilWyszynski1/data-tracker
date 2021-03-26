package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"sort"
	"strconv"
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

const spreadsheetId = "1T62QvQrrgNnKNB0JdC24qHm6bYGMgbY4usY8FLFEcec"

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
	var (
		binanceApiKey    = os.Getenv("BINANCE_API_KEY")
		binanceSecretKey = os.Getenv("BINANCE_SECRET_KEY")
	)

	bCli := binance.NewBinance(http.DefaultClient, "https://api.binance.com", binanceApiKey, []byte(binanceSecretKey))

	accInfo, err := bCli.Account(binance.AccountRequest{
		Timestamp: time.Now().Add(-time.Second),
	})
	if err != nil {
		log.Fatalf("Failed to get account info")
	}

	balance := accInfo.Balances
	sort.Slice(balance, func(i, j int) bool {
		return balance[i].Asset <= balance[j].Asset
	})

	var (
		len_    = 0
		assets  = make([]interface{}, 0)
		amounts = make([]interface{}, 0)
		ratios  = make([]interface{}, 0)
	)
	for _, balance := range accInfo.Balances {
		v, err := strconv.ParseFloat(balance.Free, 64)
		if err != nil {
			log.Printf("error occured: %s", err)
			continue
		}
		if v != 0 {
			assets = append(assets, balance.Asset)
			amounts = append(amounts, balance.Free)
			len_++
			ratio, err := bCli.SymbolTickerPrice(fmt.Sprintf("%s%s", balance.Asset, "BUSD")) // ratios based on USD price
			if err != nil {
				log.Printf("error when getting ticker price: %s\n", err)
			}
			ratios = append(ratios, ratio)
		}
	}

	var (
		date    = time.Now().Format("2006-01-02")
		entryNo = 0
	)

	vr := &sheets.ValueRange{}
	vr.Values = append(vr.Values, []interface{}{date})
	_, err = srv.Spreadsheets.Values.Update(spreadsheetId, fmt.Sprintf("Balance!A%d", 1+entryNo), vr).ValueInputOption("USER_ENTERED").Do()
	if err != nil {
		panic(err)
	}

	vr = &sheets.ValueRange{}
	{
		vr.Values = append(vr.Values, assets)
		vr.Values = append(vr.Values, amounts)
		vr.Values = append(vr.Values, ratios)
	}
	range_ := fmt.Sprintf("Balance!B%d:%s%d", 1+entryNo, string(rune('B'+len_-1)), 3+entryNo)
	fmt.Printf("insertint to: %s\n", range_)
	_, err = srv.Spreadsheets.Values.Update(spreadsheetId, range_, vr).ValueInputOption("USER_ENTERED").Do()
	if err != nil {
		panic(err)
	}
}

func insertTrades(srv *sheets.Service, bCli *binance.Client) {
	// Prints the names and majors of students in a sample spreadsheet:
	// https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit
	readRange := "A:A"
	resp, err := srv.Spreadsheets.Values.Get(spreadsheetId, readRange).Do()
	if err != nil {
		log.Fatalf("Unable to retrieve data from sheet: %v", err)
	}

	var sells []pair

	if len(resp.Values) == 0 {
		fmt.Println("No data found.")
	} else {
		for i, row := range resp.Values {
			if len(row) == 0 {
				continue
			}

			crypto := row[0].(string)
			fmt.Println(crypto)

			mtr, err := bCli.MyTrades(binance.MyTradesRequest{
				Symbol:    crypto,
				Timestamp: time.Now().Add(-time.Second),
			})
			if err != nil {
				panic(fmt.Errorf("failed to get MyTrades, %w", err))
			}

			sort.Slice(mtr.Trades, func(i, j int) bool {
				return mtr.Trades[i].Time <= mtr.Trades[j].Time
			})

			var (
				len_       = len(mtr.Trades)
				ratios     = make([]interface{}, 0, len_)
				times      = make([]interface{}, 0, len_)
				qtys       = make([]interface{}, 0, len_)
				prices     = make([]interface{}, 0, len_)
				commAssets = make([]interface{}, 0, len_)
				comms      = make([]interface{}, 0, len_)
			)
			for j, trade := range mtr.Trades {
				if !trade.Isbuyer {
					sells = append(sells, pair{int64(i), int64(j)})
					qtys = append(qtys, "-"+trade.Qty)
				} else {
					qtys = append(qtys, trade.Qty)
				}
				ratios = append(ratios, trade.Price)
				times = append(times, trade.GetTimeFormat("2006-01-02"))

				prices = append(prices, trade.Quoteqty)
				commAssets = append(commAssets, trade.Commissionasset)
				comms = append(comms, trade.Commission)

			}

			vr := &sheets.ValueRange{}
			{
				vr.Values = append(vr.Values, ratios)
				vr.Values = append(vr.Values, times)
				vr.Values = append(vr.Values, qtys)
				vr.Values = append(vr.Values, prices)
				vr.Values = append(vr.Values, commAssets)
				vr.Values = append(vr.Values, comms)

			}
			fmt.Println("values: ", ratios)
			range_ := fmt.Sprintf("B%d:%s%d", 1+i, string(rune('B'+len(ratios)-1)), 6+i)
			fmt.Printf("inserting %s to range: %s\n", crypto, range_)
			_, err = srv.Spreadsheets.Values.Update(spreadsheetId, range_, vr).ValueInputOption("USER_ENTERED").Do()
			if err != nil {
				panic(err)
			}
		}

		reqs := make([]*sheets.Request, 0, len(sells))
		const height = 5
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
						EndRowIndex:      p.x + 1 + height,
						StartColumnIndex: p.y + 1,
						StartRowIndex:    p.x,
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
