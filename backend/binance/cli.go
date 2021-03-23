package binance

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"net/url"

	"github.com/binanceBot/backend/binance/models"
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
	accountPath      = "api/v3/account"
	allOrdersPath    = "api/v3/allOrders"
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

// MyTrades returns list of completed trades
func (b Binance) MyTrades(req models.MyTradesRequest) (*models.MyTradesResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", b.base, myTradesPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, b.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, b.apiKey)

	var trades models.MyTradesResponse

	resp, err := b.h.Do(r)
	if err != nil {
		return nil, err
	}
	if err := json.NewDecoder(resp.Body).Decode(&trades.Trades); err != nil {
		return nil, err
	}

	return &trades, nil
}

func (b Binance) Account(req models.AccountRequest) (*models.AccountResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", b.base, accountPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, b.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, b.apiKey)

	var trades models.AccountResponse

	resp, err := b.h.Do(r)
	if err != nil {
		return nil, err
	}
	if err := json.NewDecoder(resp.Body).Decode(&trades); err != nil {
		return nil, err
	}

	return &trades, nil
}

func (b Binance) AllOrderList(req models.AllOrdersRequest) (*models.AllOrdersResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", b.base, allOrdersPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, b.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, b.apiKey)

	var orders models.AllOrdersResponse

	resp, err := b.h.Do(r)
	if err != nil {
		return nil, err
	}
	bo, _ := ioutil.ReadAll(resp.Body)
	fmt.Println(string(bo))
	if err := json.NewDecoder(resp.Body).Decode(&orders.Orders); err != nil {
		return nil, err
	}

	return &orders, nil
}

func (b Binance) createURL(req models.RequestInterface, parsedURL *url.URL) string {
	q := &url.Values{}
	req.EmbedData(q)

	h := hmac.New(sha256.New, b.secretKey)
	h.Write([]byte(q.Encode()))
	sha := hex.EncodeToString(h.Sum(nil))

	parsedURL.RawQuery = q.Encode() + "&signature=" + sha

	return parsedURL.String()
}
