package binance

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io/ioutil"
	"net/http"
	"net/url"
	"strconv"
)

type Binance struct {
	h         *http.Client
	base      string
	apiKey    string
	secretKey []byte
}

const (
	apiKeyHeader = "X-MBX-APIKEY"

	pingPath         = "api/v3/ping"
	exchangeInfoPath = "api/v3/exchangeInfo"
	myTradesPath     = "api/v3/myTrades"
)

func (b Binance) Ping() {
	url := fmt.Sprintf("%s/%s", b.base, pingPath)
	fmt.Println(b.h.Get(url))
}

func (b Binance) ExchangeInfo() {
	url := fmt.Sprintf("%s/%s", b.base, exchangeInfoPath)
	r, _ := b.h.Get(url)
	body, _ := ioutil.ReadAll(r.Body)
	fmt.Println(string(body))
}

/*
https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md#account-trade-list-user_data
NAME		TYPE 	MANDATORY 	DESCRIPION
===========================================
symbol		STRING	YES
startTime	LONG	NO
endTime		LONG	NO
fromId		LONG	NO			TradeId to fetch from. Default gets most recent trades.
limit		INT		NO			Default 500; max 1000.
recvWindow	LONG	NO			The value cannot be greater than 60000
timestamp	LONG	YES
*/
func (b Binance) MyTrades(req MyTradesRequest) error {
	if err := req.Validate(); err != nil {
		return err
	}

	u := fmt.Sprintf("%s/%s", b.base, myTradesPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return err
	}

	q := parsedURL.Query()
	q.Set("symbol", req.Symbol)
	q.Set("timestmap", strconv.Itoa(int(req.Timestamp.Unix())))

	h := hmac.New(sha256.New, b.secretKey)
	h.Write([]byte(q.Encode()))
	sha := hex.EncodeToString(h.Sum(nil))
	q.Set("signature", sha)

	parsedURL.RawQuery = q.Encode()

	fmt.Println(parsedURL.String())
	r, _ := http.NewRequest(http.MethodGet, parsedURL.String(), nil)
	r.Header.Set(apiKeyHeader, b.apiKey)

	resp, _ := b.h.Do(r)
	body, _ := ioutil.ReadAll(resp.Body)
	fmt.Println(string(body))

	return nil
}
