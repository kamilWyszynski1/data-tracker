package binance

import (
	"fmt"
	"net/http"
	"os"
	"testing"
	"time"
)

var (
	apiKey    = ""
	secretKey = ""
)

func init() {
	if key := os.Getenv("BINANCE_API_KEY"); key != "" {
		apiKey = key
	}
	if key := os.Getenv("BINANCE_SECRET_KEY"); key != "" {
		secretKey = key
	}
}

func TestBinance_Ping(t *testing.T) {
	b := Client{
		h:    http.DefaultClient,
		base: "https://api.binance.com",
	}
	b.Ping()
}

// func TestBinance_ExchangeInfo(t *testing.T) {
// 	b := Client{
// 		h:    http.DefaultClient,
// 		base: "https://api.binance.com",
// 	}
// 	b.ExchangeInfo()
// }
func TestBinance_MyTrades(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	fmt.Println(b.MyTrades(MyTradesRequest{
		Symbol:    "BTCUSDC",
		Timestamp: time.Now().Add(-time.Second),
	}))
}

func TestBinance_Account(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	r, err := b.Account(AccountRequest{
		Timestamp: time.Now().Add(-time.Second),
	})
	fmt.Printf("%+v\n", r)
	fmt.Println(err)
}

func TestBinance_AllOrderList(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	r, err := b.AllOrderList(AllOrdersRequest{
		Timestamp: time.Now().Add(-time.Second),
	})
	fmt.Printf("%+v\n", r)
	fmt.Println(err)
}
