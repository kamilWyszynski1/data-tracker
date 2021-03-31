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
		Symbol:     "BTCUSDC",
		RecvWindow: 10000000,
		Timestamp:  time.Now().Add(-time.Second),
	}))
}

func TestBinance_Account(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	ti, err := b.GetServerTime()
	if err != nil {
		t.Fatal(err)
	}
	r, err := b.Account(AccountRequest{
		Timestamp:  ti,
		RecvWindow: 10000000,
	})
	fmt.Printf("%+v\n", r)
	fmt.Println(err)
}

func TestBinance_GetServerTime(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	r, err := b.GetServerTime()
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

func TestClient_CurrentAveragePrice(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	r, err := b.CurrentAveragePrice("BTCBUSD")
	fmt.Printf("%+v\n", r)
	fmt.Println(err)
}

func TestClient_SymbolTickerPrice(t *testing.T) {
	b := Client{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	r, err := b.SymbolTickerPrice("BTCBUSD")
	fmt.Printf("%+v\n", r)
	fmt.Println(err)
}
