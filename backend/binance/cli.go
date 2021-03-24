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
)

type Client struct {
	h         *http.Client
	base      string
	apiKey    string
	secretKey []byte
}

func NewBinance(h *http.Client, base string, apiKey string, secretKey []byte) *Client {
	return &Client{h: h, base: base, apiKey: apiKey, secretKey: secretKey}
}

const (
	apiKeyHeader = "X-MBX-APIKEY"

	pingPath         = "api/v3/ping"
	exchangeInfoPath = "api/v3/exchangeInfo"
	myTradesPath     = "api/v3/myTrades"
	accountPath      = "api/v3/account"
	allOrdersPath    = "api/v3/allOrders"
)

func (c Client) Ping() {
	url := fmt.Sprintf("%s/%s", c.base, pingPath)
	fmt.Println(c.h.Get(url))
}

func (c Client) ExchangeInfo() {
	url := fmt.Sprintf("%s/%s", c.base, exchangeInfoPath)
	r, _ := c.h.Get(url)
	body, _ := ioutil.ReadAll(r.Body)
	fmt.Println(string(body))
}

/*
https://githuc.com/binance/binance-spot-api-docs/blob/master/rest-api.md#account-trade-list-user_data
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
func (c Client) MyTrades(req MyTradesRequest) (*MyTradesResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", c.base, myTradesPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, c.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, c.apiKey)

	var trades MyTradesResponse

	resp, err := c.h.Do(r)
	if err != nil {
		return nil, err
	}
	if err := json.NewDecoder(resp.Body).Decode(&trades.Trades); err != nil {
		return nil, err
	}

	return &trades, nil
}

func (c Client) Account(req AccountRequest) (*AccountResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", c.base, accountPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, c.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, c.apiKey)

	var trades AccountResponse

	resp, err := c.h.Do(r)
	if err != nil {
		return nil, err
	}
	if err := json.NewDecoder(resp.Body).Decode(&trades); err != nil {
		return nil, err
	}

	return &trades, nil
}

func (c Client) AllOrderList(req AllOrdersRequest) (*AllOrdersResponse, error) {
	if err := req.Validate(); err != nil {
		return nil, err
	}

	u := fmt.Sprintf("%s/%s", c.base, allOrdersPath)
	parsedURL, err := url.Parse(u)
	if err != nil {
		return nil, err
	}

	r, _ := http.NewRequest(http.MethodGet, c.createURL(req, parsedURL), nil)
	r.Header.Set(apiKeyHeader, c.apiKey)

	var orders AllOrdersResponse

	resp, err := c.h.Do(r)
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

func (c Client) createURL(req RequestInterface, parsedURL *url.URL) string {
	q := &url.Values{}
	req.EmbedData(q)

	h := hmac.New(sha256.New, c.secretKey)
	h.Write([]byte(q.Encode()))
	sha := hex.EncodeToString(h.Sum(nil))

	parsedURL.RawQuery = q.Encode() + "&signature=" + sha

	return parsedURL.String()
}
