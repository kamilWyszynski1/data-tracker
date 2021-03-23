package binance

import (
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
	b := Binance{
		h:    http.DefaultClient,
		base: "https://api.binance.com",
	}
	b.Ping()
}

// func TestBinance_ExchangeInfo(t *testing.T) {
// 	b := Binance{
// 		h:    http.DefaultClient,
// 		base: "https://api.binance.com",
// 	}
// 	b.ExchangeInfo()
// }
func TestBinance_MyTrades(t *testing.T) {
	b := Binance{
		h:         http.DefaultClient,
		base:      "https://api.binance.com",
		apiKey:    apiKey,
		secretKey: []byte(secretKey),
	}
	b.MyTrades(MyTradesRequest{
		Symbol:    "BUSDBTC",
		Timestamp: time.Now(),
	})
}
